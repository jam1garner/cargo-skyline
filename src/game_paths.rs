pub const ATMOSPHERE_CONTENTS_DIR: &str = "/atmosphere/contents";

pub fn get_plugins_path(title_id: &str) -> String {
    format!(
        "{}/{}/romfs/skyline/plugins",
        ATMOSPHERE_CONTENTS_DIR, title_id
    )
}

pub fn get_plugin_path(title_id: &str, plugin_name: &str) -> String {
    format!(
        "{}/{}/romfs/skyline/plugins/{}",
        ATMOSPHERE_CONTENTS_DIR, title_id, plugin_name
    )
}

pub fn get_game_path(title_id: &str) -> String {
    format!("{}/{}", ATMOSPHERE_CONTENTS_DIR, title_id)
}

pub fn get_subsdk_path(title_id: &str, subsdk_name: &str) -> String {
    format!(
        "{}/{}/exefs/{}",
        ATMOSPHERE_CONTENTS_DIR, title_id, subsdk_name
    )
}

pub fn get_npdm_path(title_id: &str) -> String {
    format!("{}/{}/exefs/main.npdm", ATMOSPHERE_CONTENTS_DIR, title_id)
}

pub fn get_plugin_nro_path(title_id: &str, nro_file_name: &str) -> String {
    format!(
        "{}/{}/romfs/skyline/plugins/{}",
        ATMOSPHERE_CONTENTS_DIR, title_id, nro_file_name
    )
}
