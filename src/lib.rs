use std::{ mem, fs, thread, };
use std::path::PathBuf;
use std::ffi::CString;
use std::io::prelude::*;
use std::io::{ Error, ErrorKind };
use std::sync::{ Arc, Mutex };
use std::ptr;

use winapi::{
    shared::minwindef::{ HMODULE, HINSTANCE, DWORD },
    um::winnt::{ DLL_PROCESS_ATTACH },
    ctypes::{ c_void },
    um::{ consoleapi::AllocConsole, libloaderapi::GetModuleHandleA }
};
use detour::static_detour;
use fltk::{ app::*, button::*, window::*, menu::*, dialog::* };

mod helpers;

use helpers::{ read_memory, Config };

// DllMain function that spawns main thread on DLL_PROCESS_ATTACH
#[no_mangle]
#[allow(non_snake_case)]
pub extern "stdcall" fn DllMain(
    _hinst_dll: HINSTANCE,
    attach_reason: DWORD,
    _: c_void
)
{
    match attach_reason {
        DLL_PROCESS_ATTACH => { thread::spawn(initialize); },
        _ => {},
    };
}

// Create the static detour
static_detour!{ static LoadScript: extern "win64" fn(u64, u64, u64); }
// Make mem::transmute look nice with a type definition
type FnLoadScript = extern "win64" fn(u64, u64, u64);

fn initialize() {
    //unsafe { AllocConsole(); }
    println!("Injected! Initializing...");

    let base_address: HMODULE;

    unsafe { base_address = GetModuleHandleA(ptr::null()); }
    println!("Got base address: {:#X?}", base_address as u64);

    let load_script_address: u64 = base_address as u64 + 0x4CD120;
    println!("LoadScript function address is {:#X?}", load_script_address);

    // Initialize data

    let config = Arc::new(Mutex::new(Config {
        mods_path: PathBuf::from(r"..\..\mods\"),
        mods_enabled: true,
        last_character: String::new(),
    }));


    let scripts_vec: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));

    // I honestly don't think theres a better way of doing this
    // It seems like basically just doing a clone() for each closure that would move scripts_vec into its scope - 1 is the correct solution
    let scripts_ref = scripts_vec.clone();

    // GUI
    println!("Creating GUI...");
    let ui = App::default();
    ui.set_scheme(AppScheme::Gtk);
    let mut window = Window::new(100, 100, 370, 120, "GBVS Script Hook");

    let mut choice_script_extract: Choice = Choice::new(160, 10, 200, 50, "");
    choice_script_extract.add_choice("Player 1");
    choice_script_extract.add_choice("Player 1 ETC");
    choice_script_extract.add_choice("Player 1 UNKNOWN1");
    choice_script_extract.add_choice("Player 1 UNKNOWN2");
    choice_script_extract.add_choice("Player 1 UNKNOWN3");
    choice_script_extract.add_choice("Player 1 UNKNOWN4");
    choice_script_extract.add_choice("Player 2");
    choice_script_extract.add_choice("Player 2 ETC");
    choice_script_extract.add_choice("Player 2 UNKNOWN1");
    choice_script_extract.add_choice("Player 2 UNKNOWN2");
    choice_script_extract.add_choice("Player 2 UNKNOWN3");
    choice_script_extract.add_choice("Player 2 UNKNOWN4");
    choice_script_extract.add_choice("CMN");
    choice_script_extract.add_choice("CMNEF (Mechanics)");
    choice_script_extract.add_choice("Stage");
    choice_script_extract.set_value(0);

    let mut but_extract = Button::new(10, 10, 150, 50, "Extract Script");
    let mut but_set_mod_path = Button::new(10, 60, 150, 50, "Set Mods Folder");

    but_extract.set_callback(Box::new(move || {
        let mut sfd = FileDialog::new(FileDialogType::BrowseSaveFile);
        sfd.set_filter("*.bbscript");
        sfd.show();

        let mut path = sfd.filename();
        let script_index = choice_script_extract.value() as usize;

        if !path.to_str().unwrap_or("").is_empty(){
            path.set_extension("bbscript");
            println!("EXTRACT SCRIPT CALLED\npath: {:?}\nindex: {}", path, script_index);

            if let Err(e) = save_script(path, script_index, scripts_ref.clone()) {
                alert(300, 300, format!("Could not save file! Error: {}", e).as_ref());
            };
        }
    }
    ));

    let config_ref_path = config.clone();
    but_set_mod_path.set_callback(Box::new(move || {
        let mut dir_dialog = FileDialog::new(FileDialogType::BrowseSaveDir);
        dir_dialog.show();

        let path = dir_dialog.filename();

        if !path.to_str().unwrap_or("").is_empty() && path.is_dir() {
            let mut config_lock = config_ref_path.lock().unwrap();
            println!("Acquired config lock @ line 126");
            println!("Setting mods path to: {:?}", &path);
            config_lock.mods_path = path;
        }
    }));

    window.end();
    println!("Created GUI!");

    // Hook
    println!("Creating hook...");

    let load_script_detour = move |destination: u64, script_ptr: u64, script_size: u64| {
        hook_load_script(destination, script_ptr, script_size, scripts_vec.clone(), config.clone())
    };

    unsafe {
        let orig_load_script: FnLoadScript = mem::transmute(load_script_address);
        LoadScript.initialize(orig_load_script, load_script_detour).expect("Could not initialize hook!")
        .enable().expect("Could not enable hook!");
    }
    println!("Created hook!");

    window.show();
    ui.run().unwrap();
}

fn hook_load_script(dest: u64, script_ptr: u64, script_size: u64, scripts_vec: Arc<Mutex<Vec<Vec<u8>>>>, config: Arc<Mutex<Config>>) {
    println!("Load script hook called!");
    println!("Script Pointer: `{:#8X}`\nScript Size: `{}`", script_ptr, script_size);
    let dumped_script = read_memory(script_ptr, script_size);

    let mut config_lock = config.lock().unwrap();
    println!("Acquired config lock @ line 159");
    let mut scripts = scripts_vec.lock().unwrap();
    println!("Acquired scripts lock @ line 161");
    if scripts.len() == 15 {
        scripts.clear();
    }

    scripts.push(dumped_script);
    println!("scripts dumped in scripts_vec: {}", scripts.len());

    if config_lock.mods_enabled {
        // Grab character name if Player 1 or Player 2 main script
        if scripts.len() == 1 || scripts.len() == 7 {

            unsafe {
                let jump_table_size = script_ptr as *const u32;
                let name_address = 0x8 + ((*jump_table_size + 1) * 0x24);
                
                let char_name = String::from_utf8(read_memory(script_ptr + name_address as u64, 3)).unwrap();
                println!("Got character name: {}", char_name);
                config_lock.last_character = char_name;
            }   
        }

        let last_character = config_lock.last_character.to_string();
        let mut mod_file = config_lock.mods_path.to_owned();

        match scripts.len() {
            1  => { mod_file.push(format!("{}.bbscript", last_character)); },
            2  => { mod_file.push(format!("{}_etc.bbscript", last_character)); },
            3  => {},
            4  => {},
            5  => {},
            6  => {},
            7  => { mod_file.push(format!("{}.bbscript", last_character)); },
            8  => { mod_file.push(format!("{}_etc.bbscript", last_character));},
            9  => {},
            10 => {},
            11 => {},
            12 => {},
            13 => { mod_file.push(format!("cmn.bbscript")) },
            14 => { mod_file.push(format!("cmnef.bbscript")) },
            _  => {},
        };

        
        if mod_file.is_file() {
            println!("Looking for file: {:?}", mod_file);
            if let Ok(mut file) = fs::File::open(mod_file) {
                let mut modded_script = Vec::new();
                
                if file.read_to_end(&mut modded_script).is_ok() {
                    
                    let mod_ptr = modded_script.as_ptr() as u64;
                    let mod_size = modded_script.len() as u64;
                    
                    println!("LOADING MODDED SCRIPT, NEW POINTER: {:#X}", mod_ptr);
                    mem::forget(modded_script);
                    LoadScript.call(dest, mod_ptr, mod_size);
                    return;
                }
            }
        }
    }

    LoadScript.call(dest, script_ptr, script_size);
}

fn save_script(path: PathBuf, index: usize, script_vec: Arc<Mutex<Vec<Vec<u8>>>>) -> Result<(), Error> {
    let script_read = script_vec.lock().unwrap();
    println!("Acquired scripts lock @ line 226");
    let script = script_read.get(index);
    
    if let Some(script) = script {
        let file_open = fs::File::create(path);
    
        if let Err(e) = file_open {
            return Err(e);
        }

        let mut file_path = file_open.unwrap();

        if let Err(e) = file_path.write_all(script) {
            return Err(e);
        }

    } else {
        return Err(Error::new(
            ErrorKind::Other,
            "Script not found! Have you loaded in a match yet?"
        ));
    }
    return Ok(());
}