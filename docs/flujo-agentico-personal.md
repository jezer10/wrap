# Flujo agéntico personal sobre Warp

Fecha: 2026-10-21. Estado: propuesta para revisar; implementación pendiente.

## Intención

Convertir Warp en un espacio de trabajo personal centrado en proyectos, tareas, Git y múltiples
agentes de IA. La experiencia debe ser concisa, con buenos valores predeterminados y sin exigir
configurar una plataforma de automatización antes de empezar a trabajar.

La personalización incluye comportamiento y organización del trabajo, además de apariencia.
Priorizar un recorrido completo que funcione en el uso diario y ampliarlo según la experiencia.

## Base y separación

- `linux-x86_64` conserva la base de compilación para Arch/Omarchy x86_64. Commit de referencia:
  `3b7f20e`, publicado en `origin/linux-x86_64` y comprobado mediante compilación y apertura de ventana.
- Desarrollar la funcionalidad en otra rama basada en esa referencia; su nombre está pendiente.
- Concentrar políticas, estado del flujo y adaptadores en módulos definidos, con conexiones pequeñas
  y explícitas a las vistas existentes.
- Evitar cambios innecesarios al motor del terminal, sesiones y servicios compartidos.
- Una rama no aísla arquitectónicamente los cambios: modificar un componente visual compartido sigue
  afectando a sus consumidores dentro de esa rama. La separación debe estar también en el diseño.

La GUI está escrita en Rust con WarpUI. Las vistas componen elementos como `Flex`, `Container`,
`Stack` y `Text`; apariencia y temas centralizan parte de los estilos. No existe actualmente una
capa única que permita reemplazar toda la interfaz mediante configuración.

## Dirección visual aprobada

Referencia: [proyectos y consola](design/project-console-reference.png).

- Manrope para navegación y controles; fuente monoespaciada para el terminal.
- Barra lateral compacta con proyectos locales persistidos, selector de carpeta y ajustes.
- Consola como área principal, con fondo sólido y controles discretos para sus sesiones.
- Superficies continuas, separadores suaves y translucidez limitada a la navegación.
- Empezar por proyectos y consola; incorporar agentes, Git y consumo progresivamente.
- El desenfoque del escritorio depende del compositor. La primera versión no modifica Hyprland.

Rama de implementación: `feat/project-console-ui`, basada en `linux-x86_64`.
La primera vista se implementa en `app/src/workspace/view/project_console.rs`, reutilizando
`ProjectManagementModel`, las sesiones existentes y los diálogos de Warp. Cambiar de proyecto
recupera una sesión asociada cuando existe. La selección favorece el proyecto más específico
cuando hay carpetas anidadas. Los temas existentes siguen disponibles; Light es el valor inicial
para Linux y Manrope se distribuye con su licencia OFL.

## Experiencia propuesta

La unidad principal es una tarea dentro de un proyecto. Cada tarea relaciona objetivo, rama, cambios,
ejecuciones de agentes, resultados y consumo.

Flujo general:

`Proyecto → tarea → especificación → implementación → revisión → pruebas → PR/MR`

Las etapas son opcionales. Una corrección pequeña puede pasar directamente a implementación y
revisión. Las pruebas de integración se incorporan cuando el cambio lo requiere.

En la vista de tarea mostrar:

- Rama, estado del repositorio y cambios.
- Agentes asignados, estado de sus ejecuciones y resultados.
- Etapa actual y siguiente acción disponible.
- Consumo por ejecución y total de la tarea.

La información avanzada se consulta bajo demanda, sin llenar la pantalla principal de opciones.

## Git y convenciones

Separar operaciones locales de Git de las integraciones con proveedores. Crear ramas, inspeccionar
diffs y preparar commits no debe depender del proveedor remoto. PRs/MRs requieren adaptadores a las
APIs correspondientes; el primer proveedor que se implementará queda pendiente de elección.

Convenciones iniciales:

- Ramas: `{type}/{description}`, por ejemplo `feat/conversation-history`.
- Commits: `{type}({scope}): {description}`, por ejemplo
  `feat(conversations): add conversation history`.
- Títulos de PR/MR: reutilizar inicialmente la convención de commits.
- Tipos iniciales: `feat`, `fix`, `chore`, `refactor`, `docs`, `test`.

Las convenciones son políticas del proyecto utilizadas tanto por la interfaz como por los agentes:
generación de propuestas, validación y edición antes de ejecutar la operación. Definir durante la
implementación cómo manejar commits sin scope y las convenciones existentes de cada repositorio.

## Harnesses y routing

Representar por separado la tarea o función asignada, el harness que la ejecuta y el modelo elegido.
Reutilizar sus capacidades nativas cuando estén disponibles, sin prometer equivalencia entre ellos.

Comenzar con asignaciones explícitas por función: especificación, implementación, revisión y pruebas.
El routing propone harness/modelo según esas asignaciones, capacidades y disponibilidad; muestra
el motivo y permite cambiar la elección. No introducir inicialmente otro agente solo para decidir
el routing ni cambios silenciosos de proveedor.

Para múltiples agentes, contemplar dependencias, intercambio de resultados y concurrencia limitada.
Antes de permitir escrituras paralelas, definir aislamiento mediante worktrees o una estrategia
equivalente para evitar que dos agentes modifiquen simultáneamente el mismo checkout.

## Seguimiento de consumo

El consumo es parte del flujo desde la primera versión, no una pantalla añadida al final.

- Conservar consumo por ejecución, agente, harness, modelo, tarea y proyecto.
- Distinguir tokens reportados, datos parciales y datos no disponibles.
- Distinguir coste estimado de importe facturado; mostrar origen y criterio de la estimación.
- Tratar límites de suscripción o cuota como información distinta del coste monetario.
- No contabilizar dos veces snapshots acumulativos, reintentos de captura o eventos repetidos.
- Conservar el historial necesario para inspeccionar resultados y consumo tras reiniciar.

La precisión disponible dependerá de lo que exponga cada harness. No presentar ausencia de datos
como consumo cero. La política de presupuesto y su comportamiento ante datos parciales se deben
definir antes de aplicar límites automáticos.

## Configuración mínima

Ofrecer valores personales predeterminados y excepciones por proyecto. Evitar duplicar configuración
que ya pertenece al harness, como autenticación o ajustes que este administra directamente.

Decisiones visibles iniciales:

1. Harness/modelo predeterminado.
2. Excepciones por función, solo si se desean.
3. Convenciones de ramas, commits y PRs/MRs.
4. Presupuesto y concurrencia máxima.

El formato y ubicación de persistencia quedan pendientes. Los ejemplos de configuración discutidos
son ilustrativos; todavía no existe un esquema implementado.

## Piezas existentes que se deben evaluar

La inspección inicial encontró:

- `app/src/ai/orchestration/`: configuración, validación y estado de orquestación.
- `app/src/pane_group/pane/local_harness_launch.rs`: preparación de lanzamientos de harnesses locales.
- `app/src/ai/harness_availability.rs`: disponibilidad de harnesses y modelos.
- `crates/warp_harness_usage/`: extracción y representación de consumo para Claude Code y Codex,
  incluyendo cobertura conocida, parcial o no disponible.
- `app/src/server/server_api/harness_usage/`: publicación de datos de consumo.

La presencia de estas piezas no demuestra que el recorrido funcione de forma independiente. Falta
comprobar requisitos de autenticación, servicios de Warp/Oz, flags y persistencia local. Separar
captura local de consumo de su publicación a un servidor cuando sea necesario.

## Primera entrega

Implementar un único recorrido comprobable:

1. Abrir un proyecto local y crear una tarea.
2. Proponer y crear una rama siguiendo la convención.
3. Ejecutar un harness elegido y registrar estado y resultado.
4. Revisar el diff desde la tarea.
5. Consultar consumo reportado o su indisponibilidad explícita.
6. Preparar un commit con título editable siguiendo la convención.
7. Recuperar la tarea y su historial después de reiniciar.

Después ampliar con delegación entre agentes, routing por funciones, etapas de especificación,
revisión y pruebas, e integración con PRs/MRs. No implementar varios proveedores o un editor visual
de workflows antes de validar el recorrido inicial.

## Próxima revisión

Retomar el 2026-10-22, según disponibilidad del usuario:

- Confirmar el primer harness y proveedor Git que se usarán realmente.
- Auditar el recorrido local existente y sus dependencias de servicios externos.
- Definir el aislamiento de ejecuciones y el modelo mínimo de tarea/ejecución/consumo.
- Precisar presupuesto, revisión de acciones de Git y alcance de la primera entrega.
- Crear la rama de desarrollo y empezar por el recorrido inicial.

Este documento recoge la intención y una propuesta de alcance para la siguiente sesión.
