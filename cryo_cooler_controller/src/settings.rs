use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TecInputs {
    p_coef: f32,
    i_coef: f32,
    d_coef: f32,
    set_point: f32,
    max_power: u8,
}

impl Default for TecInputs {
    fn default() -> Self {
        Self {
            p_coef: 100.0,
            i_coef: 1.0,
            d_coef: 1.0,
            set_point: 2.0,
            max_power: 100,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PersistentDataV1 {
    version: u32,
    data: Settings,
}

#[derive(Deserialize, Debug, Clone)]
struct DeserializeHelper {
    version: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Settings {
    last_port_ident: Option<PathBuf>,
    open_port_on_startup: bool,
    tec_inputs: TecInputs,
    enable_on_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_port_ident: None,
            open_port_on_startup: false,
            tec_inputs: TecInputs::default(),
            enable_on_startup: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    config_dir_path: PathBuf,
    settings: Settings,
}

const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE: &str = "cryo_settings.json";
const SETTINGS_TEMP_FILE: &str = "cryo_settings_old.json";
const SETTINGS_DIR: &str = "cryo_cooler_controller";

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            config_dir_path: "./".into(),
            settings: Settings::default(),
        }
    }
}

macro_rules! set_value {
    ($self:ident, $value:ident, $($field_path:ident).+) => {
        if $value != $self.$($field_path).+ {
            $self.$($field_path).+ = $value;
            return $self.write_to_disk();
        }
        return Ok(());
    }
}
impl AppSettings {
    fn load_settings(path: PathBuf) -> Self {
        if let Ok(file_content) = std::fs::read_to_string(path.join(SETTINGS_FILE)) {
            if let Ok(ser) = serde_json::from_str::<DeserializeHelper>(&file_content) {
                match ser.version {
                    1 => {
                        if let Ok(v1) = serde_json::from_str::<PersistentDataV1>(&file_content) {
                            return AppSettings {
                                config_dir_path: path,
                                settings: v1.data,
                            };
                        } else {
                            let _ = std::fs::rename(
                                path.join(SETTINGS_FILE),
                                path.join(format!(
                                    "cryo_settings_backup_{}.json",
                                    chrono::Utc::now().format("%Y_%m_%d_%H_%M_%S")
                                )),
                            );
                        }
                    }
                    _ => {
                        let _ = std::fs::rename(
                            path.join(SETTINGS_FILE),
                            path.join(format!(
                                "cryo_settings_backup_{}.json",
                                chrono::Utc::now().format("%Y_%m_%d_%H_%M_%S")
                            )),
                        );
                    }
                }
            }
        }

        AppSettings {
            config_dir_path: path,
            settings: Settings::default(),
        }
    }

    fn determine_settings_dir_path() -> PathBuf {
        if let Ok(r) = std::fs::exists(SETTINGS_FILE) {
            if r {
                return "./".into();
            }
        }
        if let Some(config_dir_path) = config_dir() {
            let dir = config_dir_path.join(SETTINGS_DIR);
            if let Ok(r) = std::fs::exists(&dir) {
                if r {
                    return dir;
                }
            }
            if let Ok(()) = std::fs::create_dir(&dir) {
                return dir;
            }
        }
        "./".into()
    }
    pub fn new() -> Self {
        AppSettings::load_settings(AppSettings::determine_settings_dir_path())
    }

    pub fn get_last_port_ident(&self) -> &Option<PathBuf> {
        &self.settings.last_port_ident
    }

    pub fn set_last_port_ident(&mut self, value: Option<PathBuf>) -> std::io::Result<()> {
        set_value!(self, value, settings.last_port_ident);
    }

    pub fn get_open_port_on_startup(&self) -> bool {
        self.settings.open_port_on_startup
    }

    pub fn set_open_port_on_startup(&mut self, value: bool) -> std::io::Result<()> {
        set_value!(self, value, settings.open_port_on_startup);
    }

    pub fn get_p_coef(&self) -> f32 {
        self.settings.tec_inputs.p_coef
    }

    pub fn set_p_coef(&mut self, value: f32) -> std::io::Result<()> {
        set_value!(self, value, settings.tec_inputs.p_coef);
    }

    pub fn get_i_coef(&self) -> f32 {
        self.settings.tec_inputs.i_coef
    }

    pub fn set_i_coef(&mut self, value: f32) -> std::io::Result<()> {
        set_value!(self, value, settings.tec_inputs.i_coef);
    }

    pub fn get_d_coef(&self) -> f32 {
        self.settings.tec_inputs.d_coef
    }

    pub fn set_d_coef(&mut self, value: f32) -> std::io::Result<()> {
        set_value!(self, value, settings.tec_inputs.d_coef);
    }

    pub fn get_set_point(&self) -> f32 {
        self.settings.tec_inputs.set_point
    }

    pub fn set_set_point(&mut self, value: f32) -> std::io::Result<()> {
        set_value!(self, value, settings.tec_inputs.set_point);
    }

    pub fn get_max_power(&self) -> u8 {
        self.settings.tec_inputs.max_power
    }

    pub fn set_max_power(&mut self, value: u8) -> std::io::Result<()> {
        set_value!(self, value, settings.tec_inputs.max_power);
    }

    pub fn get_enable_on_startup(&self) -> bool {
        self.settings.enable_on_startup
    }

    pub fn set_enable_on_startup(&mut self, value: bool) -> std::io::Result<()> {
        set_value!(self, value, settings.enable_on_startup);
    }

    fn write_to_disk(&mut self) -> std::io::Result<()> {
        let _ = std::fs::rename(
            self.config_dir_path.join(SETTINGS_FILE),
            self.config_dir_path.join(SETTINGS_TEMP_FILE),
        );
        match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.config_dir_path.join(SETTINGS_FILE))
        {
            Ok(out_file) => {
                let ser_data = PersistentDataV1 {
                    version: SETTINGS_VERSION,
                    data: self.settings.clone(),
                };
                if let Err(e) = serde_json::to_writer_pretty(out_file, &ser_data) {
                    let _ = std::fs::rename(
                        self.config_dir_path.join(SETTINGS_TEMP_FILE),
                        self.config_dir_path.join(SETTINGS_FILE),
                    );
                    return Err(std::io::Error::other(format!("{e}")));
                }
            }
            Err(e) => {
                let _ = std::fs::rename(
                    self.config_dir_path.join(SETTINGS_TEMP_FILE),
                    self.config_dir_path.join(SETTINGS_FILE),
                );
                return Err(e);
            }
        }
        let _ = std::fs::remove_file(self.config_dir_path.join(SETTINGS_TEMP_FILE));
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, unused)]
mod tests {

    use std::io::{Read, Write};

    use super::*;
    const DEFAULT_SETTING_PRETTY: &str = "{\n  \"version\": 1,\n  \"data\": {\n    \"last_port_ident\": null,\n    \"open_port_on_startup\": false,\n    \"tec_inputs\": {\n      \"p_coef\": 100.0,\n      \"i_coef\": 1.0,\n      \"d_coef\": 1.0,\n      \"set_point\": 2.0,\n      \"max_power\": 100\n    },\n    \"enable_on_startup\": false\n  }\n}";
    const INVALID_SETTING_PRETTY: &str = "{\n  \"version\": 1,\n  \"data\": \"invalid\"\n}";
    const OUTDATED_SETTING_PRETTY: &str = "{\n  \"version\": 0,\n  \"data\": {\n    \"last_port_ident\": null,\n    \"open_port_on_startup\": false,\n    \"tec_inputs\": {\n      \"p_coef\": 100.0,\n      \"i_coef\": 1.0,\n      \"d_coef\": 1.0,\n      \"set_point\": 2.0,\n      \"max_power\": 100\n    },\n    \"enable_on_startup\": false\n  }\n}";

    #[test]
    fn empty() {
        let test_dir = tempdir::TempDir::new("test").unwrap();
        let mut settings = AppSettings::load_settings(test_dir.path().into());
        settings.write_to_disk();

        assert!(std::fs::exists(test_dir.path().join(SETTINGS_FILE)).unwrap());
        let mut file = std::fs::File::open(test_dir.path().join(SETTINGS_FILE)).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content);
        assert_eq!(content, DEFAULT_SETTING_PRETTY);
    }

    #[test]
    fn valid() {
        let test_dir = tempdir::TempDir::new("test").unwrap();
        {
            let mut settings = AppSettings::load_settings(test_dir.path().into());
            settings.set_enable_on_startup(true);
        }
        {
            let mut settings = AppSettings::load_settings(test_dir.path().into());
            assert!(settings.get_enable_on_startup());
        }
    }

    #[test]
    fn invalid() {
        let test_dir = tempdir::TempDir::new("test").unwrap();
        {
            let mut invalid_file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(test_dir.path().join(SETTINGS_FILE))
                .unwrap();
            invalid_file.write_all(INVALID_SETTING_PRETTY.as_bytes());
        }

        let mut settings = AppSettings::load_settings(test_dir.path().into());
        settings.write_to_disk();

        let files: Vec<_> = std::fs::read_dir(test_dir.path()).unwrap().collect();
        assert_eq!(files.len(), 2);

        for dir_result in files {
            let dir_entry = dir_result.unwrap();
            let full_path = dir_entry.path();
            let filename = dir_entry.file_name();
            if filename
                .to_str()
                .unwrap()
                .starts_with("cryo_settings_backup_")
            {
                let mut file = std::fs::File::open(full_path).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content);
                assert_eq!(content, INVALID_SETTING_PRETTY);
            } else {
                assert_eq!(filename, SETTINGS_FILE);
                let mut file = std::fs::File::open(full_path).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content);
                assert_eq!(content, DEFAULT_SETTING_PRETTY);
            }
        }
    }

    #[test]
    fn old() {
        let test_dir = tempdir::TempDir::new("test").unwrap();
        {
            let mut invalid_file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(test_dir.path().join(SETTINGS_FILE))
                .unwrap();
            invalid_file.write_all(OUTDATED_SETTING_PRETTY.as_bytes());
        }

        let mut settings = AppSettings::load_settings(test_dir.path().into());
        settings.write_to_disk();

        let files: Vec<_> = std::fs::read_dir(test_dir.path()).unwrap().collect();
        assert_eq!(files.len(), 2);

        for dir_result in files {
            let dir_entry = dir_result.unwrap();
            let full_path = dir_entry.path();
            let filename = dir_entry.file_name();
            if filename
                .to_str()
                .unwrap()
                .starts_with("cryo_settings_backup_")
            {
                let mut file = std::fs::File::open(full_path).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content);
                assert_eq!(content, OUTDATED_SETTING_PRETTY);
            } else {
                assert_eq!(filename, SETTINGS_FILE);
                let mut file = std::fs::File::open(full_path).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content);
                assert_eq!(content, DEFAULT_SETTING_PRETTY);
            }
        }
    }
}
