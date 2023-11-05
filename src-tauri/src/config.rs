#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct DevicePairing {
    pub device_id: u16,
    pub transmission_type: u8,
}

impl From<DevicePairing> for antrs::device::DevicePairing {
    fn from(value: DevicePairing) -> Self {
        Self {
            device_id: value.device_id,
            transmission_type: value.transmission_type,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Pairings {
    pub heart_rate_monitor: DevicePairing,
    pub fitness_equipment: DevicePairing,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AppConfig {
    pub devices: Pairings,
}
