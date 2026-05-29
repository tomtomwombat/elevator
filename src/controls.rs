use crate::simulation::Simulation;
use crossterm::event::KeyCode;
use std::fmt;

const DEFAULT_SPEED: u64 = 1;
const DEFAULT_STATS_WINDOW: u64 = 1_000;
const DEFAULT_TRAFFIC_SCALE: f64 = 0.1;

const MAX_SPEED: u64 = 1 << 20;

pub struct Controls {
    speed: u64,
    stats_window: u64,
    traffic_scale: f64,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            speed: DEFAULT_SPEED,
            stats_window: DEFAULT_STATS_WINDOW,
            traffic_scale: DEFAULT_TRAFFIC_SCALE,
        }
    }
}

impl Controls {
    pub fn handle_input(&mut self, key: KeyCode, sims: &mut [Simulation]) {
        match key {
            KeyCode::Char('+') | KeyCode::Char('=') => self.speed = (self.speed * 2).min(MAX_SPEED),
            KeyCode::Char('-') | KeyCode::Char('_') => self.speed = (self.speed / 2).max(1),
            KeyCode::Char(',') | KeyCode::Char('<') => {
                self.stats_window = (self.stats_window / 2).max(1000);
                for s in sims.iter_mut() {
                    s.stats.reset(s.building.prev_time(), self.stats_window);
                }
            }
            KeyCode::Char('>') | KeyCode::Char('.') => {
                self.stats_window = (self.stats_window * 2).min(3600_000);
                for s in sims.iter_mut() {
                    s.stats.reset(s.building.prev_time(), self.stats_window);
                }
            }
            KeyCode::Down => {
                self.traffic_scale = (self.traffic_scale - 0.1).max(0.0);
                for s in sims.iter_mut() {
                    s.traffic.scale(self.traffic_scale);
                }
            }
            KeyCode::Up => {
                self.traffic_scale += 0.1;
                for s in sims.iter_mut() {
                    s.traffic.scale(self.traffic_scale);
                }
            }
            _ => {}
        }
    }

    pub fn stats_window(&self) -> u64 {
        self.stats_window
    }

    pub fn traffic_scale(&self) -> f64 {
        self.traffic_scale
    }

    pub fn speed(&self) -> u64 {
        self.speed
    }
}

impl fmt::Display for Controls {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            " [Q] Quit    [+/-] Speed ({}x)    [</>] Window ({}s)    [←/→] Policy    [↑/↓] Traffic Scale {:.3}    [Tab] Switch Visualization",
            self.speed(),
            self.stats_window() / 1000,
            self.traffic_scale()
        )
    }
}
