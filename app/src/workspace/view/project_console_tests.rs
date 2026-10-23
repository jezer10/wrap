use std::path::Path;
use std::time::Duration;

use instant::Instant;

use super::{TAB_RESIZE_DURATION, TabWidthTransition, project_for_directory};

#[test]
fn selects_the_most_specific_project_for_a_nested_directory() {
    let projects = [Path::new("/projects/app"), Path::new("/projects/app/tools")];

    assert_eq!(
        project_for_directory(Path::new("/projects/app/tools/src"), projects.into_iter()),
        Some(Path::new("/projects/app/tools")),
    );
}

#[test]
fn does_not_select_a_project_with_a_similar_name() {
    assert_eq!(
        project_for_directory(
            Path::new("/projects/app-backup"),
            [Path::new("/projects/app")].into_iter()
        ),
        None,
    );
}

#[test]
fn handles_an_empty_project_list() {
    assert_eq!(
        project_for_directory(Path::new("/projects/app"), std::iter::empty()),
        None
    );
}

#[test]
fn switching_tabs_mid_animation_preserves_width_and_reaches_new_target() {
    let started = Instant::now();
    let mut width = TabWidthTransition {
        from: 96.,
        target: 238.,
        started,
    };
    let halfway = started + TAB_RESIZE_DURATION / 2;
    let before = width.width_at(halfway);
    assert!(before > 96. && before < 238.);
    width.retarget(96., halfway);
    assert_eq!(width.width_at(halfway), before);
    assert_eq!(width.width_at(halfway + TAB_RESIZE_DURATION), 96.);
    assert_eq!(width.width_at(halfway + Duration::from_secs(1)), 96.);
}

#[test]
fn rerendering_same_target_does_not_restart_tab_expansion() {
    let started = Instant::now();
    let mut width = TabWidthTransition {
        from: 96.,
        target: 238.,
        started,
    };
    width.retarget(238., started + TAB_RESIZE_DURATION / 2);
    assert_eq!(width.width_at(started + TAB_RESIZE_DURATION), 238.);
}
