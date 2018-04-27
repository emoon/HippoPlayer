use service::Service;
use wrui::ffi_gen::PUPluginUI;
use wrui::wrui::PluginUi;
use std::os::raw::{c_void, c_uchar};
use std::mem::transmute;
use ffi::HippoServiceAPI;

pub trait View {
    fn new(service: &Service, ui: PluginUi) -> Self;
    fn destroy(&mut self) { }
}

pub fn create_view_instance<T: View>(service_api: *const ::ffi::HippoServiceAPI,
                                     ui: *const PUPluginUI) -> *mut c_void {
    let plugin_ui = PluginUi::new(ui);
    let service = Service::new(service_api);
    let instance = unsafe { transmute(Box::new(T::new(&service, plugin_ui))) };
    instance
}

pub fn destroy_view_instance<T: View>(ptr: *mut c_void) {
    let view: &mut T = unsafe { &mut *(ptr as *mut T) };
    view.destroy();
    let _: Box<T> = unsafe { transmute(ptr) };
    // implicitly dropped
}

#[macro_export]
macro_rules! define_view_plugin {
    ($p_name:ident, $name:expr, $version:expr, $x:ty) => {
        static $p_name: hippo_api::HippoViewPlugin = hippo_api::HippoViewPlugin {
            api_version: 0,
            name: $name as *const u8,
            version: $version as *const u8,
            create: Some(hippo_api::view::create_view_instance::<$x>),
            destroy: Some(hippo_api::view::destroy_view_instance::<$x>),
            // event: Some(hippo_api::view::event_view_instance::<$x>),
            save: None,
            load: None,
            event: None,
        };

        let ret: *const std::os::raw::c_void = unsafe { std::mem::transmute(&$p_name) };
        ret
    }
}

