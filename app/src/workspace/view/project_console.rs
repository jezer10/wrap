use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use instant::Instant;
use settings::Setting;
use warp_core::ui::design;
use warpui::r#async::Timer;
use warpui::color::ColorU;
use warpui::elements::{
    AfterLayoutContext, Align, Border, ChildAnchor, ChildView, ClippedScrollStateHandle,
    ClippedScrollable, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Dismiss,
    DispatchEventResult, Element, Empty, EventContext, EventHandler, Expanded, Fill, Flex,
    Hoverable, LayoutContext, MainAxisSize, MouseStateHandle, OffsetPositioning, PaintContext,
    ParentAnchor, ParentElement, ParentOffsetBounds, Point, PositionedElementAnchor,
    PositionedElementOffsetBounds, SavePosition, ScrollTarget, ScrollToPositionMode,
    ScrollbarWidth, SizeConstraint, Stack, Text,
};
use warpui::event::DispatchedEvent;
use warpui::fonts::{Properties, Weight};
use warpui::geometry::rect::RectF;
use warpui::geometry::vector::{Vector2F, vec2f};
use warpui::platform::Cursor;
use warpui::text_layout::ClipConfig;
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, ClipBounds, EntityId, SingletonEntity, ViewContext, ViewHandle};

use super::{
    NEW_TERMINAL_TAB_BINDING_NAME, SHOW_SETTINGS_KEYBINDING_NAME, TAB_BAR_POSITION_ID,
    TAB_CONTENT_POSITION_ID, TOGGLE_RIGHT_PANEL_BINDING_NAME, Workspace,
};
use crate::appearance::Appearance;
use crate::editor::{EditorView, Event as EditorEvent, SingleLineEditorOptions, TextOptions};
use crate::projects::ProjectManagementModel;
use crate::terminal::general_settings::GeneralSettings;
use crate::ui_components::buttons::icon_button;
use crate::ui_components::icons::Icon;
use crate::util::bindings::keybinding_name_to_display_string;
use crate::workspace::WorkspaceAction;

struct ProjectDetailsFade {
    child: Box<dyn Element>,
    started: Option<Instant>,
    background: ColorU,
}

impl Element for ProjectDetailsFade {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        self.child.layout(constraint, ctx, app)
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.child.paint(origin, ctx, app);
        let progress = self.started.map_or(1., |started| {
            (started.elapsed().as_secs_f32() / 0.25).min(1.)
        });
        if progress < 1. {
            let eased = progress * progress * (3. - 2. * progress);
            let mut background = self.background;
            background.a = (f32::from(background.a) * (1. - eased)) as u8;
            // The fade must not affect layout or cover the card's mouse targets.
            ctx.scene.start_layer(ClipBounds::ActiveLayer);
            ctx.scene
                .draw_rect_without_hit_recording(RectF::new(origin, self.child.size().unwrap()))
                .with_background(background)
                .with_corner_radius(CornerRadius::with_all(design::SURFACE_RADIUS));
            ctx.scene.stop_layer();
            ctx.repaint_after(Duration::from_millis(16));
        }
    }

    fn size(&self) -> Option<Vector2F> {
        self.child.size()
    }

    fn origin(&self) -> Option<Point> {
        self.child.origin()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        self.child.dispatch_event(event, ctx, app)
    }
}

const TAB_RESIZE_DURATION: Duration = Duration::from_millis(180);

#[derive(Clone, Copy)]
struct TabWidthTransition {
    from: f32,
    target: f32,
    started: Instant,
}

impl TabWidthTransition {
    fn width_at(&self, now: Instant) -> f32 {
        let progress = (now.duration_since(self.started).as_secs_f32()
            / TAB_RESIZE_DURATION.as_secs_f32())
        .clamp(0., 1.);
        let eased = progress * progress * (3. - 2. * progress);
        self.from + (self.target - self.from) * eased
    }

    fn retarget(&mut self, target: f32, now: Instant) {
        if self.target != target {
            self.from = self.width_at(now);
            self.target = target;
            self.started = now;
        }
    }
}

struct AnimatedConsoleTab {
    child: Box<dyn Element>,
    width: TabWidthTransition,
    reveal: Option<ClippedScrollStateHandle>,
}

impl Element for AnimatedConsoleTab {
    fn layout(
        &mut self,
        mut constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        let width = self.width.width_at(Instant::now());
        constraint.min.set_x(width);
        constraint.max.set_x(width);
        self.child.layout(constraint, ctx, app)
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.child.paint(origin, ctx, app);
        if self.width.from != self.width.target
            && self.width.started.elapsed() < TAB_RESIZE_DURATION
        {
            if let Some(scroll) = &self.reveal {
                scroll.scroll_to_position(ScrollTarget {
                    position_id: "project_console:active_tab".to_owned(),
                    mode: ScrollToPositionMode::FullyIntoView,
                });
            }
            ctx.repaint_after(Duration::from_millis(16));
        }
    }

    fn size(&self) -> Option<Vector2F> {
        self.child.size()
    }

    fn origin(&self) -> Option<Point> {
        self.child.origin()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        self.child.dispatch_event(event, ctx, app)
    }
}

#[derive(Default)]
pub(super) struct ProjectConsoleState {
    project_hover_mouse_states: RefCell<HashMap<PathBuf, MouseStateHandle>>,
    project_mouse_states: RefCell<HashMap<PathBuf, MouseStateHandle>>,
    details_path: Option<String>,
    details_pinned: bool,
    details_shown_at: Option<Instant>,
    hover_generation: u64,
    details_mouse: MouseStateHandle,
    edit_mouse: MouseStateHandle,
    save_mouse: MouseStateHandle,
    close_mouse: MouseStateHandle,
    alias_editor: Option<ViewHandle<EditorView>>,
    alias_error: Option<String>,
    projects_scroll: ClippedScrollStateHandle,
    consoles_scroll: ClippedScrollStateHandle,
    tab_widths: RefCell<HashMap<EntityId, TabWidthTransition>>,
    add_project: MouseStateHandle,
    new_console: MouseStateHandle,
    settings: MouseStateHandle,
    side_panel: MouseStateHandle,
}

impl ProjectConsoleState {
    pub(super) fn reveal_active_tab(&self) {
        self.consoles_scroll.scroll_to_position(ScrollTarget {
            position_id: "project_console:active_tab".to_owned(),
            mode: ScrollToPositionMode::FullyIntoView,
        });
    }
}

pub(super) fn project_for_directory<'a>(
    directory: &Path,
    projects: impl Iterator<Item = &'a Path>,
) -> Option<&'a Path> {
    projects
        .filter(|project| directory.starts_with(project))
        .max_by_key(|project| project.components().count())
}

fn label(text: impl Into<String>, size: f32, appearance: &Appearance) -> Text {
    Text::new(text.into(), appearance.ui_font_family(), size)
        .soft_wrap(false)
        .with_clip(ClipConfig::ellipsis())
        .with_color(appearance.theme().active_ui_text_color().into_solid())
}

fn icon(icon: Icon, appearance: &Appearance) -> Box<dyn Element> {
    ConstrainedBox::new(
        icon.to_warpui_icon(appearance.theme().active_ui_text_color())
            .finish(),
    )
    .with_width(16.)
    .with_height(16.)
    .finish()
}

fn action_tooltip(
    label: &'static str,
    binding_name: Option<&str>,
    appearance: &Appearance,
    ctx: &AppContext,
) -> Box<dyn Element> {
    let builder = appearance.ui_builder();
    if let Some(shortcut) =
        binding_name.and_then(|name| keybinding_name_to_display_string(name, ctx))
    {
        builder
            .tool_tip_with_sublabel(label.to_owned(), shortcut)
            .build()
            .finish()
    } else {
        builder.tool_tip(label.to_owned()).build().finish()
    }
}

fn icon_action(
    icon: Icon,
    state: &MouseStateHandle,
    action: WorkspaceAction,
    tooltip: &'static str,
    binding_name: Option<&str>,
    appearance: &Appearance,
    ctx: &AppContext,
) -> Box<dyn Element> {
    let tooltip = action_tooltip(tooltip, binding_name, appearance, ctx);
    icon_button(appearance, icon, false, state.clone())
        .with_tooltip(move || tooltip)
        .build()
        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
        .finish()
}

fn button(
    content: Box<dyn Element>,
    state: &MouseStateHandle,
    action: WorkspaceAction,
    selected: bool,
    tooltip: Option<(&'static str, &str)>,
    appearance: &Appearance,
    ctx: &AppContext,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let hover = design::hover_background(theme);
    let selection = design::selected_background(theme);
    let tooltip =
        tooltip.map(|(label, binding)| action_tooltip(label, Some(binding), appearance, ctx));
    Hoverable::new(state.clone(), move |mouse| {
        let background = if selected {
            selection.into()
        } else if mouse.is_hovered() {
            hover.into()
        } else {
            Fill::None
        };
        let button = Container::new(content)
            .with_horizontal_padding(12.)
            .with_vertical_padding(10.)
            .with_corner_radius(CornerRadius::with_all(design::CONTROL_RADIUS))
            .with_background(background)
            .finish();
        let mut stack = Stack::new().with_child(button);
        if mouse.is_hovered()
            && let Some(tooltip) = tooltip
        {
            stack.add_positioned_overlay_child(
                tooltip,
                OffsetPositioning::offset_from_parent(
                    vec2f(0., -8.),
                    ParentOffsetBounds::WindowByPosition,
                    ParentAnchor::TopMiddle,
                    ChildAnchor::BottomMiddle,
                ),
            );
        }
        stack.finish()
    })
    .with_cursor(Cursor::PointingHand)
    .on_click(move |ctx, _, _| ctx.dispatch_typed_action(action.clone()))
    .finish()
}

fn project_position_id(path: &str) -> String {
    format!("project_console:project:{path}")
}

fn project_name(path: &str, ctx: &AppContext) -> String {
    GeneralSettings::as_ref(ctx)
        .project_aliases
        .value()
        .get(path)
        .cloned()
        .unwrap_or_else(|| {
            Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_owned())
        })
}

impl Workspace {
    pub(super) fn hover_project(&mut self, path: &str, hovered: bool, ctx: &mut ViewContext<Self>) {
        if self.project_console.details_pinned {
            return;
        }
        self.project_console.hover_generation += 1;
        if hovered {
            if self.project_console.details_path.as_deref() == Some(path) {
                return;
            }
            let generation = self.project_console.hover_generation;
            let path = path.to_owned();
            ctx.spawn(
                Timer::after(Duration::from_millis(450)),
                move |me, _, ctx| {
                    if me.project_console.hover_generation == generation
                        && !me.project_console.details_pinned
                    {
                        me.project_console.details_path = Some(path);
                        me.project_console.details_shown_at = Some(Instant::now());
                        ctx.notify();
                    }
                },
            );
        } else {
            let generation = self.project_console.hover_generation;
            // Allow the pointer to cross from the project row into the floating card.
            ctx.spawn(
                Timer::after(Duration::from_millis(250)),
                move |me, _, ctx| {
                    if me.project_console.hover_generation == generation
                        && !me.project_console.details_pinned
                        && !me
                            .project_console
                            .details_mouse
                            .lock()
                            .unwrap()
                            .is_hovered()
                        && !me
                            .project_console
                            .details_path
                            .as_ref()
                            .is_some_and(|path| {
                                me.project_console
                                    .project_hover_mouse_states
                                    .borrow()
                                    .get(Path::new(path))
                                    .is_some_and(|mouse| mouse.lock().unwrap().is_hovered())
                            })
                    {
                        me.close_project_details(ctx);
                    }
                },
            );
        }
    }

    pub(super) fn toggle_project_details(&mut self, path: &str, ctx: &mut ViewContext<Self>) {
        if self.project_console.details_pinned
            && self.project_console.details_path.as_deref() == Some(path)
        {
            self.close_project_details(ctx);
        } else {
            self.project_console.details_path = Some(path.to_owned());
            self.project_console.details_pinned = true;
            let was_editing = self.project_console.alias_editor.take().is_some();
            self.project_console.alias_error = None;
            if was_editing {
                self.focus_active_tab(ctx);
            }
            ctx.notify();
        }
    }

    pub(super) fn close_project_details(&mut self, ctx: &mut ViewContext<Self>) {
        let was_editing = self.project_console.alias_editor.take().is_some();
        self.project_console.details_path = None;
        self.project_console.details_shown_at = None;
        self.project_console.details_pinned = false;
        self.project_console.alias_error = None;
        self.project_console.hover_generation += 1;
        if was_editing {
            self.focus_active_tab(ctx);
        }
        ctx.notify();
    }

    pub(super) fn edit_project(&mut self, path: &str, ctx: &mut ViewContext<Self>) {
        let name = project_name(path, ctx);
        let editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let mut editor = EditorView::single_line(
                SingleLineEditorOptions {
                    text: TextOptions::ui_text(Some(14.), appearance),
                    select_all_on_focus: true,
                    ..Default::default()
                },
                ctx,
            );
            editor.insert_selected_text(&name, ctx);
            editor
        });
        ctx.subscribe_to_view(&editor, |me, _, event, ctx| {
            if me.project_console.alias_editor.is_none() {
                return;
            }
            if matches!(event, EditorEvent::Enter) {
                me.save_project_alias(ctx);
            } else if matches!(event, EditorEvent::Escape) {
                me.close_project_details(ctx);
            }
        });
        self.project_console.details_path = Some(path.to_owned());
        self.project_console.details_pinned = true;
        self.project_console.alias_error = None;
        self.project_console.alias_editor = Some(editor.clone());
        ctx.focus(&editor);
        ctx.notify();
    }

    pub(super) fn save_project_alias(&mut self, ctx: &mut ViewContext<Self>) {
        let (Some(path), Some(editor)) = (
            self.project_console.details_path.clone(),
            self.project_console.alias_editor.as_ref(),
        ) else {
            return;
        };
        let alias = editor.as_ref(ctx).buffer_text(ctx).trim().to_owned();
        let result = GeneralSettings::handle(ctx).update(ctx, |settings, ctx| {
            let mut aliases = settings.project_aliases.value().clone();
            if alias.is_empty() {
                aliases.remove(&path);
            } else {
                aliases.insert(path, alias);
            }
            settings.project_aliases.set_value(aliases, ctx)
        });
        if result.is_ok() {
            self.close_project_details(ctx);
        } else {
            self.project_console.alias_error =
                Some("No se pudo guardar el seudónimo. Inténtalo de nuevo.".to_owned());
            ctx.notify();
        }
    }

    fn render_project_details(&self, path: &str, ctx: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(ctx);
        let theme = appearance.theme();
        let state = &self.project_console;
        let header = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(icon(Icon::Folder, appearance))
            .with_child(
                Expanded::new(1., label(project_name(path, ctx), 14., appearance).finish())
                    .finish(),
            )
            .with_child(icon_action(
                Icon::X,
                &state.close_mouse,
                WorkspaceAction::CloseProjectDetails,
                "Cerrar",
                None,
                appearance,
                ctx,
            ))
            .finish();
        let mut content = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(12.)
            .with_child(header)
            .with_child(
                Text::new(path.to_owned(), appearance.ui_font_family(), 12.)
                    .soft_wrap(true)
                    .with_color(theme.nonactive_ui_text_color().into_solid())
                    .finish(),
            );
        if let Some(editor) = &state.alias_editor {
            content.add_child(label("Seudónimo", 12., appearance).finish());
            content.add_child(
                Container::new(ChildView::new(editor).finish())
                    .with_uniform_padding(8.)
                    .with_corner_radius(CornerRadius::with_all(design::CONTROL_RADIUS))
                    .with_border(Border::all(1.).with_border_fill(design::control_border(theme)))
                    .finish(),
            );
            content.add_child(
                label(
                    "Déjalo vacío para usar el nombre de la carpeta.",
                    12.,
                    appearance,
                )
                .soft_wrap(true)
                .finish(),
            );
            if let Some(error) = &state.alias_error {
                content.add_child(
                    label(error.clone(), 12., appearance)
                        .soft_wrap(true)
                        .finish(),
                );
            }
            content.add_child(button(
                label("Guardar", 13., appearance).finish(),
                &state.save_mouse,
                WorkspaceAction::SaveProjectAlias,
                false,
                None,
                appearance,
                ctx,
            ));
        } else {
            let edit = Flex::row()
                .with_spacing(8.)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(icon(Icon::Pencil, appearance))
                .with_child(label("Editar proyecto", 13., appearance).finish())
                .finish();
            content.add_child(button(
                edit,
                &state.edit_mouse,
                WorkspaceAction::EditProject {
                    path: path.to_owned(),
                },
                false,
                None,
                appearance,
                ctx,
            ));
        }
        let card = ConstrainedBox::new(
            Container::new(content.finish())
                .with_uniform_padding(12.)
                .with_background(theme.background())
                .with_drop_shadow(design::popup_shadow(theme))
                .with_corner_radius(CornerRadius::with_all(design::SURFACE_RADIUS))
                .finish(),
        )
        .with_width(320.)
        .finish();
        let card = ProjectDetailsFade {
            child: card,
            started: state.details_shown_at,
            background: theme.background().into_solid(),
        }
        .finish();
        let path = path.to_owned();
        let card = EventHandler::new(card)
            .on_keydown(|ctx, _, keystroke| {
                if keystroke.is_unmodified_key("escape") {
                    ctx.dispatch_typed_action(WorkspaceAction::CloseProjectDetails);
                    DispatchEventResult::StopPropagation
                } else {
                    DispatchEventResult::PropagateToParent
                }
            })
            .finish();
        Dismiss::new(
            Hoverable::new(state.details_mouse.clone(), |_| card)
                .on_hover(move |hovered, ctx, _, _| {
                    ctx.dispatch_typed_action(WorkspaceAction::HoverProject {
                        path: path.clone(),
                        hovered,
                    });
                })
                .finish(),
        )
        .on_dismiss(|ctx, _| ctx.dispatch_typed_action(WorkspaceAction::CloseProjectDetails))
        .finish()
    }
}

impl Workspace {
    pub(super) fn render_project_console(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let state = &self.project_console;
        let directory = self
            .active_tab_pane_group()
            .as_ref(app)
            .active_session_view(app)
            .and_then(|terminal| terminal.as_ref(app).reported_session_path_if_local(app));
        let mut projects = ProjectManagementModel::as_ref(app)
            .all_projects()
            .collect::<Vec<_>>();
        projects.sort_by(|a, b| a.path.cmp(&b.path));
        let selected = directory.as_deref().and_then(|directory| {
            project_for_directory(
                directory,
                projects.iter().map(|project| Path::new(&project.path)),
            )
        });
        let sidebar_header = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Expanded::new(
                    1.,
                    label("wrap", 22., appearance)
                        .with_style(Properties::default().weight(Weight::Bold))
                        .finish(),
                )
                .finish(),
            )
            .with_child(icon_action(
                Icon::Plus,
                &state.add_project,
                WorkspaceAction::OpenRepository { path: None },
                "Añadir proyecto",
                Some("workspace:open_repository"),
                appearance,
                app,
            ))
            .finish();
        let mut project_list = Flex::column()
            .with_spacing(8.)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_main_axis_size(MainAxisSize::Min);
        let mut mouse_states = state.project_mouse_states.borrow_mut();
        let project_paths = projects
            .iter()
            .map(|project| Path::new(&project.path))
            .collect::<HashSet<_>>();
        mouse_states.retain(|path, _| project_paths.contains(path.as_path()));
        let mut hover_states = state.project_hover_mouse_states.borrow_mut();
        hover_states.retain(|path, _| mouse_states.contains_key(path));
        for project in &projects {
            let path = Path::new(&project.path);
            let is_selected = selected == Some(path);
            let name = project_name(&project.path, app);
            let row = Flex::row()
                .with_spacing(10.)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(icon(Icon::Folder, appearance))
                .with_child(Expanded::new(1., label(name, 14., appearance).finish()).finish())
                .finish();
            let open = button(
                row,
                mouse_states.entry(path.to_path_buf()).or_default(),
                WorkspaceAction::OpenRepository {
                    path: Some(project.path.clone()),
                },
                is_selected,
                None,
                appearance,
                app,
            );
            let row = open;
            let project_path = project.path.clone();
            let details_path = project.path.clone();
            project_list.add_child(
                SavePosition::new(
                    Hoverable::new(
                        hover_states.entry(path.to_path_buf()).or_default().clone(),
                        |_| row,
                    )
                    .on_hover(move |hovered, ctx, _, _| {
                        ctx.dispatch_typed_action(WorkspaceAction::HoverProject {
                            path: project_path.clone(),
                            hovered,
                        });
                    })
                    .on_right_click(move |ctx, _, _| {
                        ctx.dispatch_typed_action(WorkspaceAction::ToggleProjectDetails {
                            path: details_path.clone(),
                        });
                    })
                    .finish(),
                    &project_position_id(&project.path),
                )
                .finish(),
            );
        }
        if projects.is_empty() {
            project_list.add_child(
                Container::new(
                    label("Añade una carpeta\npara comenzar.", 12., appearance)
                        .soft_wrap(true)
                        .finish(),
                )
                .with_uniform_padding(12.)
                .finish(),
            );
        }
        let scroll = ClippedScrollable::vertical(
            state.projects_scroll.clone(),
            project_list.finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_detail().into(),
            theme.active_ui_detail().into(),
            Fill::None,
        )
        .with_overlayed_scrollbar()
        .finish();
        let settings = Flex::row()
            .with_spacing(10.)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(icon(Icon::Gear, appearance))
            .with_child(label("Ajustes", 13., appearance).finish())
            .finish();
        let sidebar = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Container::new(sidebar_header)
                    .with_padding_bottom(26.)
                    .finish(),
            )
            .with_child(
                Container::new(
                    label("Proyectos", 12., appearance)
                        .with_color(theme.nonactive_ui_text_color().into_solid())
                        .finish(),
                )
                .with_horizontal_padding(12.)
                .with_padding_bottom(12.)
                .finish(),
            )
            .with_child(Expanded::new(1., scroll).finish())
            .with_child(button(
                settings,
                &state.settings,
                WorkspaceAction::ShowSettings,
                false,
                Some(("Ajustes", SHOW_SETTINGS_KEYBINDING_NAME)),
                appearance,
                app,
            ))
            .finish();
        let sidebar = ConstrainedBox::new(
            Container::new(sidebar)
                .with_uniform_padding(14.)
                .with_background(theme.background().with_opacity(94))
                .with_border(Border::right(1.).with_border_fill(design::subtle_border(theme)))
                .finish(),
        )
        .with_width(232.)
        .finish();

        let controls_width = 2. * 32. + 3. * 12. + 2. * 8.;
        let tabs_width = app
            .element_position_by_id_at_last_frame(self.window_id, TAB_BAR_POSITION_ID)
            .map(|rect| (rect.width() - controls_width).max(0.))
            .or_else(|| {
                app.windows()
                    .platform_window(self.window_id)
                    .map(|window| (window.size().x() - 232. - controls_width).max(0.))
            })
            .unwrap_or(238.);
        let tab_count = self.tabs.len().max(1) as f32;
        let active_width = tabs_width.clamp(160., 238.);
        let inactive_width = ((tabs_width - active_width - 8. * (tab_count - 1.))
            / (tab_count - 1.).max(1.))
        .clamp(96., 156.);
        let now = Instant::now();
        let mut widths = state.tab_widths.borrow_mut();
        let tab_ids = self
            .tabs
            .iter()
            .map(|tab| tab.pane_group.id())
            .collect::<HashSet<_>>();
        widths.retain(|id, _| tab_ids.contains(id));
        let mut consoles = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(8.);
        for (index, tab) in self.tabs.iter().enumerate() {
            let name = if self.tabs.len() == 1 {
                "Consola".to_owned()
            } else {
                tab.pane_group.as_ref(app).display_title(app)
            };
            let selected = index == self.active_tab_index;
            let target_width = if selected {
                active_width
            } else {
                inactive_width
            };
            let width = widths
                .entry(tab.pane_group.id())
                .or_insert(TabWidthTransition {
                    from: inactive_width,
                    target: target_width,
                    started: now,
                });
            width.retarget(target_width, now);
            let width = *width;
            let row = Flex::row()
                .with_child(icon(Icon::Globe, appearance))
                .with_main_axis_size(MainAxisSize::Max)
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Expanded::new(
                        1.,
                        Container::new(label(name, 12., appearance).finish())
                            .with_horizontal_padding(6.)
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(icon_action(
                    Icon::X,
                    &tab.close_mouse_state,
                    WorkspaceAction::CloseTab(index),
                    "Cerrar consola",
                    (index == self.active_tab_index).then_some("workspace:close_active_tab"),
                    appearance,
                    app,
                ))
                .finish();
            let selection = design::selected_background(theme);
            let hover = design::hover_background(theme);
            let tab = Hoverable::new(tab.tab_mouse_state.clone(), move |mouse| {
                Container::new(Align::new(row).finish())
                    .with_background(if selected {
                        selection.into()
                    } else if mouse.is_hovered() {
                        hover.into()
                    } else {
                        Fill::None
                    })
                    .with_corner_radius(CornerRadius::with_all(design::CONTROL_RADIUS))
                    .with_border(
                        if selected {
                            Border::all(1.)
                        } else {
                            Border::left(1.)
                        }
                        .with_border_fill(design::subtle_border(theme)),
                    )
                    .with_horizontal_padding(8.)
                    .finish()
            })
            .with_defer_events_to_children()
            .on_middle_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CloseTab(index));
            });
            let tab = if selected {
                tab.finish()
            } else {
                tab.with_cursor(Cursor::PointingHand)
                    .on_mouse_down(move |ctx, _, _| {
                        ctx.dispatch_typed_action(WorkspaceAction::ActivateTab(index));
                    })
                    .finish()
            };
            let tab = AnimatedConsoleTab {
                child: ConstrainedBox::new(tab).with_height(32.).finish(),
                width,
                reveal: selected.then(|| state.consoles_scroll.clone()),
            }
            .finish();
            consoles.add_child(if selected {
                SavePosition::new(tab, "project_console:active_tab").finish()
            } else {
                tab
            });
        }
        let consoles = ConstrainedBox::new(
            ClippedScrollable::horizontal(
                state.consoles_scroll.clone(),
                consoles.finish(),
                ScrollbarWidth::Custom(3.),
                theme.nonactive_ui_detail().into(),
                theme.active_ui_detail().into(),
                Fill::None,
            )
            .with_overlayed_scrollbar()
            .with_cross_axis_scroll()
            .finish(),
        )
        .with_height(32.)
        .finish();
        let consoles = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(12.)
            .with_child(
                ConstrainedBox::new(consoles)
                    .with_max_width(tabs_width)
                    .finish(),
            )
            .with_child(
                ConstrainedBox::new(
                    Align::new(icon_action(
                        Icon::Plus,
                        &state.new_console,
                        WorkspaceAction::AddTerminalTab {
                            hide_homepage: true,
                        },
                        "Nueva consola",
                        Some(NEW_TERMINAL_TAB_BINDING_NAME),
                        appearance,
                        app,
                    ))
                    .finish(),
                )
                .with_width(32.)
                .with_height(32.)
                .finish(),
            )
            .with_child(Expanded::new(1., Empty::new().finish()).finish())
            .with_child(
                ConstrainedBox::new(
                    Align::new(icon_action(
                        Icon::LayoutAlt01,
                        &state.side_panel,
                        WorkspaceAction::ToggleRightPanel,
                        "Abrir panel lateral",
                        Some(TOGGLE_RIGHT_PANEL_BINDING_NAME),
                        appearance,
                        app,
                    ))
                    .finish(),
                )
                .with_width(32.)
                .with_height(32.)
                .finish(),
            )
            .finish();
        let pane_group = self.active_tab_pane_group().as_ref(app);
        let panel_maximized = pane_group.right_panel_open && pane_group.is_right_panel_maximized;
        let mut console_content = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        if !panel_maximized {
            console_content.add_child(
                Expanded::new(1., ChildView::new(self.active_tab_pane_group()).finish()).finish(),
            );
        }
        if pane_group.right_panel_open {
            let panel = ChildView::new(&self.right_panel_view).finish();
            if panel_maximized {
                console_content.add_child(Expanded::new(1., panel).finish());
            } else {
                console_content.add_child(Self::render_panel_separator(app));
                console_content.add_child(panel);
            }
        }
        let content = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                SavePosition::new(
                    Container::new(consoles)
                        .with_horizontal_padding(8.)
                        .with_vertical_padding(10.)
                        .with_border(
                            Border::bottom(1.).with_border_fill(design::subtle_border(theme)),
                        )
                        .finish(),
                    TAB_BAR_POSITION_ID,
                )
                .finish(),
            )
            .with_child(
                Expanded::new(
                    1.,
                    SavePosition::new(console_content.finish(), TAB_CONTENT_POSITION_ID).finish(),
                )
                .finish(),
            )
            .finish();
        let content = Container::new(content)
            .with_background(theme.background().with_opacity(100))
            .finish();
        let content = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(sidebar)
            .with_child(Expanded::new(1., content).finish())
            .finish();
        let mut stack = Stack::new().with_child(content);
        if let Some(path) = &state.details_path {
            stack.add_positioned_overlay_child(
                self.render_project_details(path, app),
                OffsetPositioning::offset_from_save_position_element(
                    project_position_id(path),
                    vec2f(12., 0.),
                    PositionedElementOffsetBounds::WindowByPosition,
                    PositionedElementAnchor::TopRight,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        stack.finish()
    }
}

#[cfg(test)]
#[path = "project_console_tests.rs"]
mod tests;
