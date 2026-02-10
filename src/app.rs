use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: { title: "Camera Test" },
                body = <View> {
                    flow: Down,
                    align: { x: 0.5, y: 0.0 },
                    padding: 20,
                    spacing: 20,

                    <Label> {
                        draw_text: {
                            text_style: { font_size: 16.0 },
                            color: #fff
                        }
                        text: "Robius Camera Test"
                    }

                    capture_button = <Button> {
                        text: "Capture Photo"
                        draw_text: { text_style: { font_size: 14.0 } }
                    }

                    status_label = <Label> {
                        draw_text: {
                            text_style: { font_size: 12.0 },
                            color: #aaa
                        }
                        text: "Press the button to capture a photo"
                    }

                    captured_image = <Image> {
                        width: 300,
                        height: 300,
                        fit: Biggest,
                        visible: false,
                    }
                }
            }
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    capture_in_progress: bool,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.ui.button(ids!(capture_button)).clicked(&actions) {
            if self.capture_in_progress {
                self.ui.label(ids!(status_label)).set_text(cx, "Capture already in progress...");
                return;
            }

            self.capture_in_progress = true;
            self.ui.label(ids!(status_label)).set_text(cx, "Opening camera...");

            // Check availability first
            if !robius_camera::is_available() {
                self.ui.label(ids!(status_label)).set_text(cx, "Camera not available on this device");
                self.capture_in_progress = false;
                return;
            }

            // Capture photo
            let result = robius_camera::capture_photo(
                robius_camera::CameraPosition::Back,
                move |result| {
                    // This callback runs on a different thread, we need to signal the UI
                    match result {
                        Ok(photo) => {
                            log!("Photo captured: {}x{}, {} bytes",
                                photo.width(),
                                photo.height(),
                                photo.jpeg_data().len()
                            );
                            Cx::post_action(CameraResult::Success {
                                width: photo.width(),
                                height: photo.height(),
                                data: photo.into_jpeg_data(),
                            });
                        }
                        Err(robius_camera::Error::Cancelled) => {
                            log!("Photo capture cancelled");
                            Cx::post_action(CameraResult::Cancelled);
                        }
                        Err(e) => {
                            log!("Photo capture error: {:?}", e);
                            Cx::post_action(CameraResult::Error(format!("{:?}", e)));
                        }
                    }
                },
            );

            if let Err(e) = result {
                self.ui.label(ids!(status_label)).set_text(cx, &format!("Failed to open camera: {:?}", e));
                self.capture_in_progress = false;
            }
        }

        // Handle camera results
        for action in actions {
            match action.downcast_ref::<CameraResult>() {
                Some(CameraResult::Success { width, height, data }) => {
                    self.capture_in_progress = false;
                    self.ui.label(ids!(status_label)).set_text(
                        cx,
                        &format!("Captured: {}x{}, {} bytes", width, height, data.len()),
                    );

                    // Load the JPEG into the Image widget
                    let image = self.ui.image(ids!(captured_image));
                    if let Err(e) = image.load_jpg_from_data(cx, data) {
                        log!("Failed to load image: {:?}", e);
                        self.ui.label(ids!(status_label)).set_text(
                            cx,
                            &format!("Failed to load image: {:?}", e),
                        );
                    } else {
                        // Make image visible
                        image.set_visible(cx, true);
                    }
                    self.ui.redraw(cx);
                }
                Some(CameraResult::Cancelled) => {
                    self.capture_in_progress = false;
                    self.ui.label(ids!(status_label)).set_text(cx, "Capture cancelled");
                }
                Some(CameraResult::Error(msg)) => {
                    self.capture_in_progress = false;
                    self.ui.label(ids!(status_label)).set_text(cx, &format!("Error: {}", msg));
                }
                Some(CameraResult::None) | None => {}
            }
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

#[derive(Clone, Debug, DefaultNone)]
pub enum CameraResult {
    None,
    Success { width: u32, height: u32, data: Vec<u8> },
    Cancelled,
    Error(String),
}
