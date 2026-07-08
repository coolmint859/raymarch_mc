use glam::Vec3;

use crate::game::{EnvironmentUniform, colors::*};

pub struct VoxelWorld {
    in_game_time: f32,
    day_length: f32,
    dusk_y_transtion: f32,
    night_y_transition: f32,
    is_paused: bool,
}

impl VoxelWorld {
    pub fn new(day_length: f32)-> Self {
        Self {
            in_game_time: 12.0, // starts at noon
            day_length,
            dusk_y_transtion: 0.3,
            night_y_transition: -0.3,
            is_paused: false,
        }
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        println!("Is Paused: {}", self.is_paused);
    }

    pub fn update(&mut self, dt: f32, is_step: bool) {
        if !self.is_paused || is_step {
            let time_modifer = 24.0 / (self.day_length * 60.0);
            self.in_game_time = (self.in_game_time + dt * time_modifer) % 24.0;
            // println!("day_time: {}", self.in_game_time);
        }
    }

    pub fn calc_environment(&self) -> EnvironmentUniform {
        let angle = (self.in_game_time / 24.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;

        let sun_x = angle.cos();
        let sun_y = angle.sin();
        let sun_z = -0.3;

        let sun_dir = Vec3::new(sun_x, sun_y, sun_z).normalize();
        let twilight_peak = 0.0;

        let (sky_zen, sky_hor, sun_col, sun_int) = if sun_y > self.dusk_y_transtion {    
            (DAY_ZENITH, DAY_HORIZON, DAY_SUN, sun_y.clamp(0.0, 1.0) * 1.5)
        } else if sun_y >= twilight_peak {
            let t = (sun_y - twilight_peak) / (self.dusk_y_transtion - twilight_peak);
            
            let zen = glam::Vec3::lerp(DUSK_ZENITH, DAY_ZENITH, t);
            let hor = glam::Vec3::lerp(DUSK_HORIZON, DAY_HORIZON, t);
            let col = glam::Vec3::lerp(DUSK_SUN, DAY_SUN, t);

            (zen, hor, col, t * 0.5)
        } else if sun_y >= self.night_y_transition {
            let t = (sun_y - self.night_y_transition) / (0.0 - self.night_y_transition);
            
            let zen = glam::Vec3::lerp(NIGHT_ZENITH, DUSK_ZENITH, t);
            let hor = glam::Vec3::lerp(NIGHT_HORIZON, DUSK_HORIZON, t);
            let col = glam::Vec3::lerp(NIGHT_SUN, DUSK_SUN, t);

            (zen, hor, col, t * 0.5)
        } else {
            (NIGHT_ZENITH, NIGHT_HORIZON, NIGHT_HORIZON, 0.0)
        };

        EnvironmentUniform {
            sun_dir: [sun_dir.x, sun_dir.y, sun_dir.z, 0.0],
            sun_color: [sun_col.x, sun_col.y, sun_col.z, sun_int],
            sky_zenith: [sky_zen.x, sky_zen.y, sky_zen.z, 0.0],
            sky_horizon: [sky_hor.x, sky_hor.y, sky_hor.z, 0.0]
        }
    }
}