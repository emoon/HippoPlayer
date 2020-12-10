use miniaudio::{Device, Devices};
use messages::*;
use logger::*;

use crate::plugin_handler::DecoderPlugin;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::{Mutex};
use std::fs::OpenOptions;

use std::io::Write;

use crate::service_ffi::PluginService;
//use ringbuf::{Consumer, Producer, RingBuffer};
use ringbuf::{Producer, RingBuffer};

const DEFAULT_DEVICE_NAME: &str = "Default Sound Device";

#[derive(Clone)]
pub struct HippoPlayback {
    plugin_user_data: u64,
    plugin: DecoderPlugin,
    is_paused: bool,
}

struct DataCallback {
    players: Mutex<Vec<HippoPlayback>>,
    mix_buffer: Vec<f32>,
    read_index: usize,
    frames_decoded: usize,
}

pub struct Instance {
    _plugin_user_data: u64,
    _plugin: DecoderPlugin,
    pub write_stream: Producer<Box<[u8]>>,
}

impl HippoPlayback {
    pub fn start_with_file(
        plugin: &DecoderPlugin,
        plugin_service: &PluginService,
        filename: &str,
    ) -> Option<(HippoPlayback, Instance)> {
        let c_filename;
        let subsong_index;
        // Find subsong separator
        // TODO: store subsong index instead?
        if let Some(separator) = filename.find('|') {
            // create filename without separator
            c_filename = CString::new(&filename[..separator]).unwrap();
            subsong_index = *&filename[separator + 1..].parse::<i32>().unwrap();
        } else {
            c_filename = CString::new(filename).unwrap();
            subsong_index = 0i32;
        }

        let user_data =
            unsafe { ((plugin.plugin_funcs).create)(plugin_service.get_c_service_api()) } as u64;
        let ptr_user_data = user_data as *mut c_void;
        //let frame_size = (((plugin.plugin_funcs).frame_size)(ptr_user_data)) as usize;
        let open_state =
            unsafe { ((plugin.plugin_funcs).open)(ptr_user_data, c_filename.as_ptr(), subsong_index) };

        if open_state < 0 {
            return None;
        }

        let rb = RingBuffer::<Box<[u8]>>::new(256);
        let (prod, _cons) = rb.split();

        Some((
            HippoPlayback {
                plugin_user_data: user_data,
                plugin: plugin.clone(),
                is_paused: false,
                //_read_stream: cons,
            },
            Instance {
                write_stream: prod,
                _plugin_user_data: user_data,
                _plugin: plugin.clone(),
            },
        ))
    }
}
pub struct HippoAudio {
    pub device_name: String,
    data_callback: *mut c_void,
    output_device: Option<Device>,
    output_devices: Option<Devices>,
    pub playbacks: Vec<Instance>,
}

unsafe extern "C" fn data_callback(
    device_ptr: *mut miniaudio::ma_device,
    output_ptr: *mut c_void,
    _input_ptr: *const c_void,
    samples_to_read: u32,
) {
    let samples_to_read = (samples_to_read * 2) as usize;
    let output = std::slice::from_raw_parts_mut(output_ptr as *mut f32, samples_to_read as usize);
    let data: &mut DataCallback = std::mem::transmute((*device_ptr).pUserData);
    let playback;

    {
        // miniaudio will clear the buffer so we don't have to do it here
        let t = data.players.lock().unwrap();
        if t.len() == 0 {
            return;
        }

        playback = t[0].clone();
    }

    if playback.is_paused {
        return;
    }

    // if we have no frames decoded, call the callback
    if data.frames_decoded == 0 {
		data.frames_decoded = ((playback.plugin.plugin_funcs).read_data)(
			playback.plugin_user_data as *mut c_void,
			data.mix_buffer.as_mut_ptr() as *mut _, (samples_to_read / 2) as u32) as usize;
    }

    // if we have decoded enough data we can just copy it  
	if (data.read_index + samples_to_read) <= data.frames_decoded {
        output.copy_from_slice(&data.mix_buffer[data.read_index..data.read_index + samples_to_read]);
        data.read_index += samples_to_read;
    } else {
	    // else we need to copy what we have left, decode new frame(s) and put that into the output buffer 
	    let diff = data.frames_decoded - data.read_index;

        if diff != 0 {
            let read_end = data.read_index + diff;
			// copy the remaining stored data
			output[0..diff].copy_from_slice(&data.mix_buffer[data.read_index..read_end]);
        }

        let mut write_offset = diff; 

        // Start produce new frames to fill up the whole buffer
        loop {
            let data_left = samples_to_read - write_offset;

            let frame_count = ((playback.plugin.plugin_funcs).read_data)(
				playback.plugin_user_data as *mut c_void,
				data.mix_buffer.as_mut_ptr() as *mut _, (samples_to_read / 2) as u32) as usize;

            // if we have decoded more data than we need just copy
            // the bit we need and update the read pointer
            if frame_count >= data_left {
                output[write_offset..].copy_from_slice(&data.mix_buffer[0..data_left]);
                data.read_index = data_left;
                data.frames_decoded = frame_count; 
                break;
            } else {
                // if here we haven't filled up the buffer just yet, copy what we have and processed to decode another frame
                let offset_end = write_offset + frame_count;
                output[write_offset..offset_end].copy_from_slice(&data.mix_buffer[0..frame_count]);
                write_offset += frame_count;
            }
        }
    }
}

impl HippoAudio {
    pub fn new() -> HippoAudio {
        // This is a bit hacky so it can be shared with the device and HippoAudio
        let data_callback = Box::new(DataCallback {
			players: Mutex::new(Vec::<HippoPlayback>::new()),
			mix_buffer: vec![0.0; 4800 * 2],
			read_index: 0,
			frames_decoded: 0,
        });
        
        HippoAudio {
            device_name: DEFAULT_DEVICE_NAME.to_owned(),
            data_callback: Box::into_raw(data_callback) as *mut c_void,
            output_devices: None,
            output_device: None,
            playbacks: Vec::new(),
        }
    }

    pub fn stop(&mut self) {
        let data_callback: &DataCallback = unsafe { std::mem::transmute(self.data_callback) };
        let mut t = data_callback.players.lock().unwrap();
        t.clear();
        self.playbacks.clear();
    }

    fn select_output_device(&mut self, msg: &HippoSelectOutputDevice) -> Result<(), miniaudio::Error> {
        let name = msg.name().unwrap();
        self.init_device(name)?;
        self.device_name = name.to_owned();
        Ok(())
    }

    fn replay_output_devices(&self) -> Option<Box<[u8]>> {
        let output_devices = self.output_devices.as_ref()?;

        let mut builder = messages::FlatBufferBuilder::new_with_capacity(8192);
        let mut out_ent = Vec::with_capacity(output_devices.devices.len());

        let device_name = builder.create_string(&self.device_name);

        for dev in &output_devices.devices {
            let device_name = builder.create_string(&dev.name);

            let desc = HippoOutputDevice::create(
                &mut builder,
                &HippoOutputDeviceArgs {
                    name: Some(device_name),
                    min_channels: dev.min_channels as i32,
                    max_channels: dev.max_channels as i32,
                    min_sample_rate: dev.min_sample_rate as i32,
                    max_sample_rate: dev.max_channels as i32,
                },
            );

            out_ent.push(desc);
        }

        let devices_vec = builder.create_vector(&out_ent);

        let added_devices = HippoReplyOutputDevices::create(
            &mut builder,
            &HippoReplyOutputDevicesArgs {
                current_device: Some(device_name),
                devices: Some(devices_vec),
            },
        );

        Some(HippoMessage::create_def(
            builder,
            MessageType::reply_output_devices,
            added_devices.as_union_value(),
        ))
    }

    fn request_select_song(&mut self, msg: &HippoMessage) -> Option<Box<[u8]>> {
        let select_song = msg.message_as_request_select_song().unwrap();
        let pause = select_song.pause_state();
        let force = select_song.force();

        let data_callback: &DataCallback = unsafe { std::mem::transmute(self.data_callback) };
        let mut t = data_callback.players.lock().unwrap();

        if t.len() == 1 {
            t[0].is_paused = pause;

            if force {
                t[0].is_paused = false;
            }
        }

        None
    }

    ///
    /// Handle incoming events
    ///
    pub fn event(&mut self, msg: &HippoMessage) -> Option<Box<[u8]>> {
        match msg.message_type() {
            MessageType::request_select_song => self.request_select_song(msg),
            MessageType::request_output_devices => self.replay_output_devices(),
            MessageType::select_output_device => {
                trace!("Trying to select new output from UI");
                let select_output = msg.message_as_select_output_device().unwrap();
                if let Err(e) = self.select_output_device(&select_output) {
                    error!("Unable to select output device {:#?}", e);
                }
                None
            }
            _ => None,
        }
    }

    pub fn init_devices(&mut self) -> Result<(), miniaudio::Error> {
        self.output_devices = Some(Devices::new()?);
        Ok(())
    }

    fn init_default_device(&mut self) -> Result<(), miniaudio::Error> {
        let context = self.output_devices.as_ref().unwrap().context;

        self.output_device = Some(Device::new(
            data_callback,
            self.data_callback,
            context,
            None)?);

        Ok(())
    }

    pub fn close_device(&mut self) {
        if let Some(ref mut device) = self.output_device.as_ref() {
            device.close();
        }

        self.output_device = None;
    }

    pub fn init_device(&mut self, playback_device: &str) -> Result<(), miniaudio::Error> {
        // Try to init output devices if we have none.
        if self.output_devices.is_none() {
            self.output_devices = Some(Devices::new()?);
        }

        let output_devices = self.output_devices.as_ref().unwrap();
        let context = output_devices.context;

        if playback_device == DEFAULT_DEVICE_NAME {
            self.close_device();
            self.init_default_device()?;
        } else {
            for device in &output_devices.devices {
                let device_id = device.device_id;
                if device.name == playback_device {
                    self.close_device();
                    self.output_device = Some(Device::new(
                        data_callback,
                        self.data_callback,
                        context,
                        Some(&device_id))?);
                    break;
                }
            }
        }

        self.output_device.as_ref().unwrap().start()
    }

    //pub fn pause(&mut self) {
    //    self.audio_sink.pause();
    //}

    //pub fn play(&mut self) {
    //   self.audio_sink.play();
    //}

    pub fn start_with_file(
        &mut self,
        plugin: &DecoderPlugin,
        service: &PluginService,
        filename: &str,
    ) -> bool {

        if self.output_device.is_none() || self.output_devices.is_none() {
            error!("Unable to play {} because system has no audio device(s)", filename);
            return false;
        }

        // TODO: Do error checking
        let playback = HippoPlayback::start_with_file(plugin, service, filename);

        if let Some(pb) = playback {
            let data_callback: &DataCallback = unsafe { std::mem::transmute(self.data_callback) };
            let mut t = data_callback.players.lock().unwrap();

            if t.len() == 1 {
                t[0] = pb.0;
            } else {
                t.push(pb.0);
            }

            self.playbacks.push(pb.1);

            return true;
        }

        return false;
    }
}
