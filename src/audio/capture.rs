use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use crossbeam_channel::Sender;

#[derive(Clone)]
pub struct AudioDevice {
    pub name: String,
}

/// Which channel(s) to use from a multi-channel device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    /// Mix all channels down to mono.
    All,
    /// Use a specific channel (0-indexed).
    Single(u16),
}

impl ChannelMode {
    pub fn display(&self, max_channels: u16) -> String {
        match self {
            ChannelMode::All => format!("All ({} ch)", max_channels),
            ChannelMode::Single(ch) => format!("Channel {}", ch + 1),
        }
    }
}

pub struct AudioCapture {
    host: Host,
    devices: Vec<Device>,
    stream: Option<Stream>,
}

impl AudioCapture {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let devices: Vec<Device> = host
            .input_devices()
            .map(|devs| devs.collect())
            .unwrap_or_default();

        Self {
            host,
            devices,
            stream: None,
        }
    }

    pub fn list_devices(&self) -> Vec<AudioDevice> {
        self.devices
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let name = d
                    .description()
                    .map(|desc| desc.name().to_string())
                    .unwrap_or_else(|_| format!("Device {}", i));
                AudioDevice { name }
            })
            .collect()
    }

    pub fn default_device_index(&self) -> Option<usize> {
        let default_id = self
            .host
            .default_input_device()
            .and_then(|d| d.id().ok());

        if let Some(id) = default_id {
            self.devices
                .iter()
                .position(|d| d.id().ok().as_ref() == Some(&id))
        } else if !self.devices.is_empty() {
            Some(0)
        } else {
            None
        }
    }

    /// Query the number of input channels available on a device.
    pub fn device_channel_count(&self, device_index: usize) -> u16 {
        self.devices
            .get(device_index)
            .and_then(|d| d.default_input_config().ok())
            .map(|cfg| cfg.channels())
            .unwrap_or(1)
    }

    pub fn start(
        &mut self,
        device_index: usize,
        sample_rate: u32,
        channel_mode: ChannelMode,
        sender: Sender<Vec<f32>>,
    ) -> Result<(), String> {
        self.stop();

        let device = self
            .devices
            .get(device_index)
            .ok_or("Invalid device index")?;

        let total_channels = self.device_channel_count(device_index);

        let config = StreamConfig {
            channels: total_channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono = extract_channel(data, total_channels, channel_mode);
                    let _ = sender.try_send(mono);
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }
}

/// Extract a single channel or mix all channels from interleaved audio data.
fn extract_channel(data: &[f32], total_channels: u16, mode: ChannelMode) -> Vec<f32> {
    let ch = total_channels as usize;
    if ch <= 1 {
        return data.to_vec();
    }

    let frame_count = data.len() / ch;

    match mode {
        ChannelMode::All => {
            // Mix all channels to mono by averaging
            let mut mono = Vec::with_capacity(frame_count);
            for frame in 0..frame_count {
                let offset = frame * ch;
                let sum: f32 = data[offset..offset + ch].iter().sum();
                mono.push(sum / ch as f32);
            }
            mono
        }
        ChannelMode::Single(channel) => {
            let channel = channel as usize;
            if channel >= ch {
                // Fallback to first channel if out of range
                data.iter().step_by(ch).copied().collect()
            } else {
                data[channel..].iter().step_by(ch).copied().collect()
            }
        }
    }
}
