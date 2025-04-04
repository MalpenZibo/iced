use crate::core::mouse;
use crate::core::window::Id;
use crate::core::{Point, Size};
use crate::graphics::Compositor;
use crate::program::{DefaultStyle, Program, State};

use iced_futures::core::Element;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use winit::monitor::MonitorHandle;

#[allow(missing_debug_implementations)]
pub struct WindowManager<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub(crate) aliases: BTreeMap<winit::window::WindowId, Id>,
    entries: BTreeMap<Id, Window<P, C>>,
}

impl<P, C> WindowManager<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        id: Id,
        window: Arc<dyn winit::window::Window>,
        application: &P,
        compositor: &mut C,
        exit_on_close_request: bool,
        resize_border: u32,
    ) -> &mut Window<P, C> {
        let state = State::new(application, id, window.as_ref());
        let viewport_version = state.viewport_version();
        let physical_size = state.physical_size();
        let surface = compositor.create_surface(
            window.clone(),
            physical_size.width,
            physical_size.height,
        );
        let renderer = compositor.create_renderer();

        let _ = self.aliases.insert(window.id(), id);

        let drag_resize_window_func = super::drag_resize::event_func(
            window.as_ref(),
            resize_border as f64 * window.scale_factor(),
        );

        let _ = self.entries.insert(
            id,
            Window {
                raw: window,
                state,
                viewport_version,
                exit_on_close_request,
                drag_resize_window_func,
                surface,
                renderer,
                mouse_interaction: mouse::Interaction::None,
                prev_dnd_destination_rectangles_count: 0,
                resize_enabled: false,
                redraw_requested: false,
            },
        );

        self.entries
            .get_mut(&id)
            .expect("Get window that was just inserted")
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn first(&self) -> Option<&Window<P, C>> {
        self.entries.first_key_value().map(|(_id, window)| window)
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (Id, &mut Window<P, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn get(&self, id: Id) -> Option<&Window<P, C>> {
        self.entries.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Window<P, C>> {
        self.entries.get_mut(&id)
    }

    pub fn ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.entries.keys().cloned()
    }

    pub fn get_mut_alias(
        &mut self,
        id: winit::window::WindowId,
    ) -> Option<(Id, &mut Window<P, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get_mut(id)?))
    }

    pub fn last_monitor(&self) -> Option<MonitorHandle> {
        self.entries.values().last()?.raw.current_monitor()
    }

    pub fn remove(&mut self, id: Id) -> Option<Window<P, C>> {
        let window = self.entries.remove(&id)?;
        let _ = self.aliases.remove(&window.raw.id());

        Some(window)
    }
}

impl<P, C> Default for WindowManager<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) enum Frame {
    None,
    Waiting,
    Ready,
}

pub(crate) type ViewFn<M, T, R> = Arc<
    Box<dyn Fn() -> Option<Element<'static, M, T, R>> + Send + Sync + 'static>,
>;

#[allow(missing_debug_implementations)]
pub struct Window<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub raw: Arc<dyn winit::window::Window>,
    pub(crate) state: State<P>,
    pub viewport_version: u64,
    pub exit_on_close_request: bool,
    pub drag_resize_window_func: Option<
        Box<
            dyn FnMut(
                &dyn winit::window::Window,
                &winit::event::WindowEvent,
            ) -> bool,
        >,
    >,
    pub prev_dnd_destination_rectangles_count: usize,
    pub mouse_interaction: mouse::Interaction,
    pub surface: C::Surface,
    pub renderer: P::Renderer,
    pub resize_enabled: bool,
    pub(crate) redraw_requested: bool,
}

impl<P, C> Window<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub fn position(&self) -> Option<Point> {
        self.raw
            .inner_position()
            .ok()
            .map(|position| position.to_logical(self.raw.scale_factor()))
            .map(|position| Point {
                x: position.x,
                y: position.y,
            })
    }

    pub fn size(&self) -> Size {
        let size = self.raw.surface_size().to_logical(self.raw.scale_factor());

        Size::new(size.width, size.height)
    }

    pub fn request_redraw(&mut self) {
        if !self.redraw_requested {
            self.redraw_requested = true;
            self.raw.request_redraw();
        }
    }

    // pub fn with_view<T>(
    //     &self,
    //     f: impl Fn(&ViewFn<P::Message, P::Message, P::Message>) -> T,
    // ) -> Option<T>
    // where
    //     P::Message: 'static,
    // {
    //     self.view_fn.as_ref().and_then(|m| {
    //         let g = m.lock().unwrap();
    //         g.downcast_ref().map(|v| f(v))
    //     })
    // }
}
