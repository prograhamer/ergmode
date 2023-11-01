#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct DevicePairing {
    pub device_id: u16,
    pub transmission_type: u8,
}

impl Into<antrs::device::DevicePairing> for DevicePairing {
    fn into(self) -> antrs::device::DevicePairing {
        antrs::device::DevicePairing {
            device_id: self.device_id,
            transmission_type: self.transmission_type,
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
    pub network_key: [u8; 8],
    pub devices: Pairings,
}
