use crate::rvd::{Display, DisplayType, RvdDisplay};
use native::api::{Monitor, NativeApiTemplate, Window, WindowId};
use std::collections::HashMap;

pub struct DisplayInfoStore {
    monitors: Vec<Monitor>,
    windows: HashMap<WindowId, Window>,
}

impl DisplayInfoStore {
    pub fn new<N: NativeApiTemplate>(native: &mut N) -> Result<Self, N::Error> {
        let monitors = native.monitors()?;
        let windows = native
            .windows()?
            .into_iter()
            .map(|window| (window.id, window))
            .collect::<HashMap<_, _>>();

        Ok(Self { monitors, windows })
    }

    pub fn gen_display_info(&self, display: Display) -> Option<RvdDisplay> {
        let info = match display {
            Display::Monitor(id) => {
                let monitor = self.monitors.iter().find(|monitor| monitor.id == id)?;

                RvdDisplay {
                    native_id: id,
                    name: monitor.name.clone(),
                    display_type: DisplayType::Monitor,
                    width: monitor.width as u16,
                    height: monitor.height as u16,
                }
            }
            Display::Window(id) => {
                let window = self.windows.get(&id)?;

                RvdDisplay {
                    native_id: id,
                    name: window.name.clone(),
                    display_type: DisplayType::Window,
                    width: window.width as u16,
                    height: window.height as u16,
                }
            }
        };

        Some(info)
    }
}
