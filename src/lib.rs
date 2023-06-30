use std::{ffi::CStr, fs, ops::Shl, path::PathBuf};

use toml_edit::{Document, Item, Table};
use windows::Win32::System::Threading::{
    GetCurrentProcess, SetPriorityClass, SetProcessAffinityMask, ABOVE_NORMAL_PRIORITY_CLASS,
    BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    REALTIME_PRIORITY_CLASS,
};

mod plugin_api;

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn OBSEPlugin_Query(
    _obse: Option<&plugin_api::OBSEInterface>,
    info: Option<&mut plugin_api::PluginInfo>,
) -> bool {
    let Some(info) = info else {
        return false;
    };

    info.info_version = plugin_api::PLUGIN_INFO_VERSION;
    const NAME: &[u8] = b"CPU Affinity\0";
    info.name = Some(CStr::from_ptr(NAME.as_ptr().cast()));
    info.version = 1;

    true
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn OBSEPlugin_Load(obse: Option<&plugin_api::OBSEInterface>) -> bool {
    let default_config = include_str!("cpu_affinity.toml");
    let default_config = default_config.parse::<Document>().unwrap();

    let config_path = PathBuf::from("./Data/OBSE/plugins/cpu_affinity.toml");

    if !config_path.exists() && fs::write(&config_path, default_config.to_string()).is_err() {
        eprintln!("Can't access the config file.");
        return false;
    }

    let config = fs::read_to_string(&config_path);
    let Ok(config) = config else {
        eprintln!("Can't access the config file.");
        return false;
    };

    let Ok(mut config) = config.parse::<Document>() else {
        eprintln!("Can't parse the config file.");
        return false;
    };

    let mut config_changed = false;

    // some tedious error handling

    let cpu = toml_get_or_insert_with(
        &mut config,
        "cpu",
        |i| i.is_table(),
        &default_config,
        &mut config_changed,
    )
    .as_table_mut()
    .unwrap();

    let default_config_cpu = default_config["cpu"].as_table().unwrap();

    let affinity = toml_get_or_insert_with(
        cpu,
        "affinity",
        |i| i.is_array(),
        default_config_cpu,
        &mut config_changed,
    )
    .as_array()
    .unwrap()
    .clone();

    let editor = toml_get_or_insert_with(
        cpu,
        "editor",
        |i| i.is_bool(),
        default_config_cpu,
        &mut config_changed,
    )
    .as_bool()
    .unwrap();

    let priority = toml_get_or_insert_with(
        cpu,
        "priority",
        |i| i.is_integer(),
        default_config_cpu,
        &mut config_changed,
    )
    .as_integer()
    .unwrap();

    if let Some(obse) = obse {
        if obse.is_editor != 0 && !editor {
            return false;
        }
    }

    let mut affinity_map = 0;
    let mut auto_affinity = false;
    for core_id in affinity {
        let core_id = match core_id.as_integer() {
            Some(core_id) => {
                if core_id == -1 {
                    auto_affinity = true;
                    break;
                }
                u32::try_from(core_id).unwrap_or(u32::MAX)
            }
            None => u32::MAX,
        };

        if let Some(bit) = 1usize.checked_shl(core_id) {
            affinity_map |= bit
        }
    }

    let handle = GetCurrentProcess();
    if !handle.is_invalid() {
        eprintln!("Can't get current process handle.");
        return false;
    }

    if auto_affinity {
        let core_count = num_cpus::get();
        if core_count >= 8 {
            affinity_map = 0b10101010usize;
        } else if core_count >= 4 {
            affinity_map = 0b1111usize.shl(core_count - 4);
        } else {
            affinity_map = 0;
        }
    }
    if affinity_map == 0 {
        println!("Affinity setting disabled");
    } else {
        let succeeded = SetProcessAffinityMask(handle, affinity_map);
        match succeeded.as_bool() {
            true => {
                println!("Set process affinity to 0x{:X}", affinity_map);
            }
            false => {
                eprintln!("Failed to set process affinity.");
            }
        }
    }

    let (priority_class, priority_name) = match priority {
        0 => (IDLE_PRIORITY_CLASS, "Idle"),
        1 => (BELOW_NORMAL_PRIORITY_CLASS, "Below normal"),
        2 => (NORMAL_PRIORITY_CLASS, "Normal"),
        3 => (ABOVE_NORMAL_PRIORITY_CLASS, "Above normal"),
        4 => (HIGH_PRIORITY_CLASS, "High"),
        5 => (REALTIME_PRIORITY_CLASS, "Realtime"),
        _ => (NORMAL_PRIORITY_CLASS, "Normal"),
    };

    let succeeded = SetPriorityClass(handle, priority_class);
    match succeeded.as_bool() {
        true => {
            println!("Set process priority to {}", priority_name);
        }
        false => {
            eprintln!("Failed to set process priority.");
        }
    }

    if config_changed {
        _ = fs::write(&config_path, config.to_string());
    }

    true
}

fn toml_get_or_insert_with<'a, P>(
    table: &'a mut Table,
    key: &str,
    predicate: P,
    default: &Table,
    modified: &mut bool,
) -> &'a mut Item
where
    P: FnOnce(&Item) -> bool,
{
    match table.entry(key) {
        toml_edit::Entry::Occupied(mut item) => {
            if !predicate(item.get()) {
                *modified = true;
                item.insert(default[key].clone());
            }

            item.into_mut()
        }
        toml_edit::Entry::Vacant(item) => item.insert(default[key].clone()),
    }
}
