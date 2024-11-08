use base64::engine::general_purpose;
use base64::Engine;
use std::sync::OnceLock;
use std::{
    env, io,
    path::{Path, PathBuf},
};

use ini::Ini;

use crate::epic::{EpicError, EpicErrorKind};

const LAST_KNOWN_DECRYPTION_KEY: &'static str = "A09C853C9E95409BB94D707EADEFA52E";
pub const DECRYPTION_KEYS_API: &'static str = "https://api.legendary.gl/v1/version.json";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LegendaryResponse {
    pub egl_config: EglConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EglConfig {
    pub data_keys: Vec<String>,
}

pub static DECRYPTION_KEYS: OnceLock<Vec<String>> = OnceLock::new();

pub async fn get_decryption_keys() {
    if DECRYPTION_KEYS.get().is_none() {
        if let Ok(response) = reqwest::get(DECRYPTION_KEYS_API).await {
            if let Ok(body) = response.text().await {
                if let Ok(data) = serde_json::from_str::<LegendaryResponse>(&body) {
                    let _ = DECRYPTION_KEYS.set(data.egl_config.data_keys);
                    return ()
                }
            }
        }
    }

    let _ = DECRYPTION_KEYS.set(vec![LAST_KNOWN_DECRYPTION_KEY.to_string()]);
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RememberMeEntry {
    #[serde(rename = "Region")]
    pub region: String, //prod
    #[serde(rename = "Email")]
    pub email: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "LastName")]
    pub last_name: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "bHasPasswordAuth")]
    pub b_has_password_auth: bool,
}

impl RememberMeEntry {
    pub fn to_base64(&self) -> Result<String, Box<dyn std::error::Error>> {
        let data: Vec<RememberMeEntry> = vec![self.clone()];

        let json = serde_json::to_string(&data)?;
        let base64 = general_purpose::STANDARD.encode(json);

        Ok(base64)
    }
}

pub fn epic_get_game_user_settings_path() -> std::io::Result<PathBuf> {
    let local_appdata = match env::var("localappdata") {
        Ok(local_appdata) => PathBuf::from(local_appdata),
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Failed to find LocalAppdata env var",
            ));
        }
    };

    let game_user_settings_path = local_appdata.join("EpicGamesLauncher\\Saved\\Config\\Windows\\GameUserSettings.ini");

    if !game_user_settings_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to find GameUserSettings.ini",
        ));
    }

    dbg!(&game_user_settings_path);

    Ok(game_user_settings_path)
}

pub(crate) fn epic_get_game_user_settings_handle(path: &Path) -> std::io::Result<Ini> {
    let ini_file: Ini = match Ini::load_from_file(path) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("Error occured : {}", error);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Failed to load GameUserSettings.ini",
            ));
        }
    };

    Ok(ini_file)
}

pub fn epic_get_remember_me_raw_data() -> std::io::Result<String> {
    let path = epic_get_game_user_settings_path()?;

    let ini_file = epic_get_game_user_settings_handle(&path)?;

    if let Some(remember_me) = ini_file.section(Some("RememberMe")) {
        if let Some(enable) = remember_me.get("Enable") {
            if enable == "False" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "RememberMe is disabled !",
                ));
            }
        }

        if let Some(data) = remember_me.get("Data") {
            if data.len() != 0 {
                return Ok(data.to_string());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Failed to find refresh token",
    ))
}

fn parse_data(data: &str) -> Result<RememberMeEntry, Box<dyn std::error::Error>> {
    let data = serde_json::from_str::<Vec<RememberMeEntry>>(data)?;

    match data.iter().find(|x| x.region == "Prod") {
        Some(account) => return Ok(account.clone()),
        None => return Err("Failed to find account".into()),
    }
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum GatherRememberMeDataError {
    IoError,
    DecodeError,
    Utf8EncodingError,
    DecryptError,
    ParsingError,
    Unknown,
}

impl std::fmt::Display for GatherRememberMeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GatherRememberMeDataError::IoError => "IoError",
                GatherRememberMeDataError::DecodeError => "DecodeError",
                GatherRememberMeDataError::Utf8EncodingError => "Utf8EncodingError",
                GatherRememberMeDataError::DecryptError => "DecryptError",
                GatherRememberMeDataError::ParsingError => "ParsingError",
                GatherRememberMeDataError::Unknown => "Unknown",
            }
        )?;

        Ok(())
    }
}

impl std::error::Error for GatherRememberMeDataError {}

pub fn epic_get_remember_me_data() -> Result<RememberMeEntry, GatherRememberMeDataError> {
    let base64_data = epic_get_remember_me_raw_data().map_err(|_| GatherRememberMeDataError::IoError)?;

    dbg!(&base64_data);

    if base64_data.len() == 0 {
        return Err(GatherRememberMeDataError::IoError);
    }

    let raw_data: Vec<u8> = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|_| GatherRememberMeDataError::DecodeError)?;
    if raw_data.len() == 0 {
        return Err(GatherRememberMeDataError::DecodeError);
    }

    if raw_data[0] as char == '[' {
        let str = String::from_utf8(raw_data)
            .map_err(|_| GatherRememberMeDataError::Utf8EncodingError)?;

        dbg!(&str);

        return parse_data(&str).map_err(|_| GatherRememberMeDataError::ParsingError);
    } else {
        if let Some(keys) = DECRYPTION_KEYS.get() {
            dbg!(keys);
            let content = keys
                .iter()
                .map(|key| parse_data(&crate::decrypt::decrypt_content(&raw_data, &key)))
                .find(|result| result.is_ok());
          
          dbg!(&content);
            
            if let Some(remember_me_data) = content {
                dbg!(&remember_me_data);
                let data = remember_me_data.unwrap();
                if data.token.len() != 0 && data.display_name.len() != 0 {
                    return Ok(data);
                }
            }
        }
    }

    return Err(GatherRememberMeDataError::Unknown);
}

pub fn epic_set_remember_me_data(entry: RememberMeEntry) -> Result<(), EpicError> {
    let data = entry.to_base64().map_err(|_| {
        EpicError::new(
            EpicErrorKind::EncodingError,
            Some("Failed to encode data to base64"),
        )
    })?;

    let path = epic_get_game_user_settings_path().map_err(|_| {
        EpicError::new(
            EpicErrorKind::IoError,
            Some("Failed to find GameUserSettings.ini path"),
        )
    })?;

    let mut ini_file = epic_get_game_user_settings_handle(&path).map_err(|_| {
        EpicError::new(
            EpicErrorKind::IoError,
            Some("Failed to open GameUserSettings.ini"),
        )
    })?;

    ini_file
        .with_section(Some("RememberMe"))
        .set("Data", data)
        .set("Enable", "True");

    ini_file.write_to_file(path).map_err(|_| {
        EpicError::new(
            EpicErrorKind::IoError,
            Some("Failed to write to GameUserSettings.ini"),
        )
    })?;

    Ok(())
}
