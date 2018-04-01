use dynamic_reload::{DynamicReload, Lib, Symbol, Search, PlatformName};
use walkdir::{WalkDir, DirEntry};
use std::os::raw::{c_int, c_void, c_char};
use std::sync::Arc;
use std::ffi::CString;
use wrui::ffi_gen::{PU, PUWidget};
use std::ffi::{CStr};
use std::borrow::Cow;

use service::{PluginService, CHippoServiceAPI};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct CHippoPlaybackPlugin {
    pub api_version: u64,
    pub probe_can_play: extern "C" fn(data: *const u8, data_size: u32, buffer: *const i8, total_size: u64) -> i32,
    pub supported_extensions: extern "C" fn() -> *mut c_char,
    pub create: extern "C" fn(service_api: *const CHippoServiceAPI) -> *mut c_void,
    pub destroy: extern "C" fn(user_data: *mut c_void) -> c_int,
    pub open: extern "C" fn(user_data: *mut c_void, buffer: *const i8) -> c_int,
    pub close: extern "C" fn(user_data: *mut c_void) -> c_int,
    pub read_data: extern "C" fn(user_data: *mut c_void, dest: *mut u8, size: u32) -> c_int,
    pub seek: extern "C" fn(user_data: *mut c_void, ms: c_int) -> c_int,
    pub private_data: u64,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct CHippoViewPlugin {
    pub api_version: u64,
    pub name: *const u8, 
    pub version: *const u8, 
    pub create: extern "C" fn(service_api: *const CHippoServiceAPI, ui: *const PU, window: PUWidget) -> *mut c_void,
    pub destroy: extern "C" fn(user_data: *mut c_void) -> c_int,
    pub event: extern "C" fn(event: u32),
}

#[derive(Clone)]
pub struct DecoderPlugin {
    pub plugin: Arc<Lib>,
    pub plugin_path: String,
    pub plugin_funcs: CHippoPlaybackPlugin,
}

#[derive(Clone)]
pub struct ViewPlugin {
    pub plugin: Arc<Lib>,
    pub plugin_path: String,
    pub plugin_funcs: CHippoViewPlugin,
}

pub struct ViewPluginInstance {
	pub plugin: ViewPlugin,
	pub user_data: u64,
	pub window: PUWidget,
}

#[cfg(target_os="macos")]
pub fn get_plugin_ext() -> &'static str {
    "dylib"
}

#[cfg(target_os="linux")]
pub fn get_plugin_ext() -> &'static str {
    "so"
}

#[cfg(target_os="windows")]
pub fn get_plugin_ext() -> &'static str {
    "dll"
}

impl DecoderPlugin {
    pub fn probe_can_play(&self, data: &[u8], buffer_len: usize, filename: &str, file_size: u64) -> bool {
        let c_filename = CString::new(filename).unwrap();
        let res = ((self.plugin_funcs).probe_can_play)(data.as_ptr(), buffer_len as u32, c_filename.as_ptr(), file_size);

        match res {
            0 => true,
            _ => false,
        }
    }
}

impl ViewPlugin {
	pub fn get_name<'a>(&'a self) -> Cow<'a, str> {
        //let res = unsafe { self.plugin_funcs.name };
        unsafe { CStr::from_ptr(self.plugin_funcs.name as *const i8).to_string_lossy() }
	}

	pub fn create_instance(&self, pu: *const PU, plugin_service: &PluginService, window: PUWidget) -> ViewPluginInstance {
		let user_data = ((self.plugin_funcs).create)(
			plugin_service.get_c_service_api(), 
			pu,
			window);

		ViewPluginInstance {
			plugin: self.clone(),
			user_data: user_data as u64,
			window,
		}
	}
}

pub struct Plugins<'a> {
    pub decoder_plugins: Vec<DecoderPlugin>,
    pub view_plugins: Vec<ViewPlugin>,
    pub plugin_handler: DynamicReload<'a>,
}

impl <'a> Plugins<'a> {
    pub fn new() -> Plugins<'a> {
        Plugins {
            decoder_plugins: Vec::new(),
            view_plugins: Vec::new(),
            plugin_handler: DynamicReload::new(Some(vec!["."]), None, Search::Default),
        }
    }

    fn add_plugin_lib(&mut self, name: &str, plugin: &Arc<Lib>) {
        let func: Result<Symbol<extern "C" fn() -> *const CHippoPlaybackPlugin>, ::std::io::Error> = unsafe {
            plugin.lib.get(b"hippo_playback_plugin\0")
        };

        if let Ok(fun) = func {
            println!("Found playback plugin with callback data {:?}", fun());

            self.decoder_plugins.push(DecoderPlugin {
                plugin: plugin.clone(),
                plugin_path: name.to_owned(),
                plugin_funcs: unsafe { (*fun()).clone() },
            });
        }

        let func: Result<Symbol<extern "C" fn() -> *const CHippoViewPlugin>, ::std::io::Error> = unsafe {
            plugin.lib.get(b"hippo_view_plugin\0")
        };

        if let Ok(fun) = func {
            println!("Found view plugin with callback data {:?}", fun());

            self.view_plugins.push(ViewPlugin {
                plugin: plugin.clone(),
                plugin_path: name.to_owned(),
                plugin_funcs: unsafe { (*fun()).clone() },
            });
        }
    }

    fn check_file_type(entry: &DirEntry) -> bool {
        let path = entry.path();

        if path.ends_with(::wrui::get_wrui_name()) {
            false
        } else if let Some(ext) = path.extension() {
            ext == get_plugin_ext()
        } else {
            false
        }
    }

    fn internal_add_plugins_from_path(&mut self, path: &str) {
        for entry in WalkDir::new(path).max_depth(1) {
            if let Ok(t) = entry {
                if Self::check_file_type(&t) {
                    self.add_plugin(t.path().to_str().unwrap());
                    //println!("{}", t.path().display());
                }
            }
        }
    }

    pub fn add_plugins_from_path(&mut self) {
        self.internal_add_plugins_from_path("plugins");
        self.internal_add_plugins_from_path(".");
    }

    pub fn add_plugin(&mut self, name: &str) {
        match self.plugin_handler.add_library(name, PlatformName::No) {
            Ok(lib) => self.add_plugin_lib(name, &lib),
            Err(e) => {
                println!("Unable to load dynamic lib, err {:?}", e);
                return;
            }
        }
    }

}

