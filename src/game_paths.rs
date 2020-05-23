pub fn get_plugin_path(title_id: &str) -> String {
    format!("/atmosphere/contents/{}/romfs/skyline/plugins", title_id)
}

pub fn get_game_path(title_id: &str) -> String {
    format!("/atmosphere/contents/{}", title_id)
}

pub fn get_subsdk_path(title_id: &str, subsdk_name: &str) -> String {
    format!("atmosphere/contents/{}/exefs/{}", title_id, subsdk_name)
}

pub fn get_npdm_path(title_id: &str) -> String {
    format!("atmosphere/contents/{}/exefs/main.npdm", title_id)
}

pub fn get_plugin_nro_path(title_id: &str, nro_file_name: &str) -> String {
    format!("atmosphere/contents/{}/romfs/skyline/plugins/{}", title_id, nro_file_name)
}
