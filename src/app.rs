use glium::{
    glutin::{
        event::{ModifiersState, MouseScrollDelta, WindowEvent},
        event_loop::EventLoopProxy,
    }
};
use image::{io::Reader as ImageReader};
use imgui::*;
use imgui_glium_renderer::{Renderer};
use std::{
    fs, io::Cursor, path::Path, process::Command, thread,
};

use super::{image_view::ImageView, vec2::Vec2, UserEvent};

macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = min!($($z),*);
        if $x < y {
            $x
        } else {
            y
        }
    }}
}

const TOP_BAR_SIZE: f32 = 19.0;

pub struct App {
    pub image_view: Option<ImageView>,
    size: Vec2<f32>,
    proxy: EventLoopProxy<UserEvent>,
    error_visible: bool,
    error_message: String,
    modifiers: ModifiersState,
    mouse_position: Vec2<f32>,
}

impl App {
    pub fn update(
        &mut self,
        ui: &mut Ui,
        display: &glium::Display,
        _renderer: &mut Renderer,
        window_event: Option<&WindowEvent>,
        user_event: Option<&UserEvent>,
    ) -> bool {
        let mut exit = false;
        {
            let dimensions = display.get_framebuffer_dimensions();
            self.size = Vec2::new(dimensions.0 as f32, dimensions.1 as f32)
        }

        if let Some(event) = user_event {
            match event {
                UserEvent::ImageLoaded(image) => {
                    self.image_view = Some(ImageView::new(display, image.clone()));
                    let view = self.image_view.as_mut().unwrap();

                    let scaling = min!(
                        self.size.x() / view.size.x(),
                        (self.size.y() - TOP_BAR_SIZE) / view.size.y()
                    );
                    view.scale = min!(scaling, 1.0);
                    view.position = self.size / 2.0;
                }
                UserEvent::ImageError(error) => {
                    self.error_visible = true;
                    self.error_message = error.clone();
                }
            };
        }

        if let Some(event) = window_event {
            match event {
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse_position.set_x(position.x as f32);
                    self.mouse_position.set_y(position.y as f32);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(ref mut image) = self.image_view {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => *y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };

                        let old_scale = image.scale;
                        image.scale += image.scale * scroll as f32 / 10.0;

                        let new_size = image.scaled();
                        if new_size.x() < 100.0 || new_size.y() < 100.0 {
                            image.scale = old_scale;
                        } else {
                            let mouse_to_center = image.position - self.mouse_position;
                            image.position -=
                                mouse_to_center * (old_scale - image.scale) / old_scale;
                        }
                    }
                }
                WindowEvent::ModifiersChanged(state) => self.modifiers = *state,
                WindowEvent::DroppedFile(path) => load_image(self.proxy.clone(), path),
                WindowEvent::ReceivedCharacter(character) => {
                    let keycode = character.to_ascii_lowercase() as u32;
                    match keycode {
                        15 if self.modifiers.ctrl() => open_load_image(self.proxy.clone()),
                        23 if self.modifiers.ctrl() => exit = true,
                        14 if self.modifiers.ctrl() => new_window(),
                        _ => (),
                    }
                }
                _ => (),
            };
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left) {
            if let Some(ref mut image) = self.image_view {
                let delta = ui.mouse_drag_delta(imgui::MouseButton::Left);
                let delta = Vec2::new(delta[0] as f32, delta[1] as f32);
                image.position += delta;
                ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
            }
        }

        if let Some(ref mut image) = self.image_view {
            let image_size = image.scaled();
            let mut window_size = self.size;
            window_size.set_y(window_size.y() - TOP_BAR_SIZE);

            if image_size.x() < window_size.x() {
                if image.position.x() - image_size.x() / 2.0 < 0.0 {
                    image.position.set_x(image_size.x() / 2.0);
                }

                if image.position.x() + image_size.x() / 2.0 > window_size.x() {
                    image.position.set_x(window_size.x() - image_size.x() / 2.0);
                }
            } else {
                if image.position.x() - image_size.x() / 2.0 > 0.0 {
                    image.position.set_x(image_size.x() / 2.0);
                }

                if image.position.x() + image_size.x() / 2.0 < window_size.x() {
                    image.position.set_x(window_size.x() - image_size.x() / 2.0);
                }
            }

            if image_size.y() < window_size.y() {
                if image.position.y() - image_size.y() / 2.0 < TOP_BAR_SIZE {
                    image.position.set_y((image_size.y() / 2.0) + TOP_BAR_SIZE);
                }

                if image.position.y() + image_size.y() / 2.0 - TOP_BAR_SIZE > window_size.y() {
                    image
                        .position
                        .set_y((window_size.y() - image_size.y() / 2.0) + TOP_BAR_SIZE);
                }
            } else {
                if image.position.y() - image_size.y() / 2.0 > TOP_BAR_SIZE {
                    image.position.set_y(image_size.y() / 2.0 + TOP_BAR_SIZE);
                }

                if image.position.y() + image_size.y() / 2.0 < window_size.y() + TOP_BAR_SIZE {
                    image
                        .position
                        .set_y((window_size.y() - image_size.y() / 2.0) + TOP_BAR_SIZE);
                }
            }
        }

        let styles = ui.push_style_vars(&[
            StyleVar::WindowPadding([10.0, 10.0]),
            StyleVar::FramePadding([0.0, 4.0]),
            StyleVar::ItemSpacing([5.0, 10.0]),
        ]);

        ui.main_menu_bar(|| {
            ui.menu(im_str!("File"), true, || {
                if MenuItem::new(im_str!("New Window"))
                    .shortcut(im_str!("Ctrl + N"))
                    .build(&ui)
                {
                    new_window();
                }

                if MenuItem::new(im_str!("Open"))
                    .shortcut(im_str!("Ctrl + O"))
                    .build(&ui)
                {
                    open_load_image(self.proxy.clone());
                }

                if MenuItem::new(im_str!("Exit"))
                    .shortcut(im_str!("Ctrl + W"))
                    .build(&ui)
                {
                    exit = true;
                }
            });
        });

        if self.error_visible {
            let mut exit = false;
            let message = self.error_message.clone();
            Window::new(im_str!("Error"))
                .size([250.0, 100.0], Condition::Always)
                .position_pivot([0.5, 0.5])
                .position(
                    [self.size.x() / 2.0, self.size.y() / 2.0],
                    Condition::Appearing,
                )
                .resizable(false)
                .focus_on_appearing(false)
                .always_use_window_padding(true)
                .focused(true)
                .opened(&mut self.error_visible)
                .build(ui, || {
                    ui.text(message);
                    if ui.button(im_str!("Ok"), [50.0, 30.0]) {
                        exit = true;
                    }
                });

            if exit {
                self.error_visible = false;
            }
        }

        styles.pop(&ui);
        exit
    }

    pub fn new(proxy: EventLoopProxy<UserEvent>, size: [f32; 2]) -> Self {
        App {
            image_view: None,
            size: Vec2::new(size[0], size[1]),
            proxy,
            error_visible: false,
            error_message: String::new(),
            modifiers: ModifiersState::empty(),
            mouse_position: Vec2::default(),
        }
    }
}

fn new_window() {
    Command::new(std::env::current_exe().unwrap())
        .spawn()
        .unwrap();
}

fn open_load_image(proxy: EventLoopProxy<UserEvent>) {
    thread::spawn(move || {
        if let Some(file) = tinyfiledialogs::open_file_dialog("Open", "", None) {
            load_image(proxy, file);
        }
    });
}

pub fn load_image(proxy: EventLoopProxy<UserEvent>, path: impl AsRef<Path>) {
    let path_buf = path.as_ref().to_path_buf();
    thread::spawn(move || {
        let file = fs::read(path_buf);
        let bytes = match file {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ =
                    proxy.send_event(UserEvent::ImageError(String::from("Unable to read file")));
                return;
            }
        };

        let format = match image::guess_format(&bytes) {
            Ok(format) => format,
            Err(_) => {
                let _ = proxy.send_event(UserEvent::ImageError(String::from("Unknown format")));
                return;
            }
        };

        let image = match ImageReader::with_format(Cursor::new(&bytes), format).decode() {
            Ok(image) => image.into_rgba16(),
            Err(_) => {
                let _ = proxy.send_event(UserEvent::ImageError(String::from(
                    "Unable to decode image",
                )));
                return;
            }
        };

        let _ = proxy.send_event(UserEvent::ImageLoaded(image));
    });
}
