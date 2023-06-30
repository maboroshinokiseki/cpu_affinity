#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OBSEInterface {
    pub obse_version: u32,
    pub oblivion_version: u32,
    pub editor_version: u32,
    pub is_editor: u32,
    pub register_command: Option<unsafe extern "C" fn(info: *mut std::ffi::c_void) -> bool>,
    pub set_opcode_base: Option<unsafe extern "C" fn(opcode: u32)>,
    pub query_interface: Option<unsafe extern "C" fn(id: u32) -> *mut std::ffi::c_void>,
    pub get_plugin_handle: Option<unsafe extern "C" fn() -> u32>,
    pub register_typed_command:
        Option<unsafe extern "C" fn(info: *mut std::ffi::c_void, retnType: u8) -> bool>,
    pub get_oblivion_directory: Option<unsafe extern "C" fn() -> *const std::ffi::c_char>,
    pub get_plugin_loaded:
        Option<unsafe extern "C" fn(pluginName: *const std::ffi::c_char) -> bool>,
    pub get_plugin_version:
        Option<unsafe extern "C" fn(pluginName: *const std::ffi::c_char) -> u32>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PluginInfo {
    pub info_version: u32,
    pub name: Option<&'static std::ffi::CStr>,
    pub version: u32,
}

pub const PLUGIN_INFO_VERSION: u32 = 3;
