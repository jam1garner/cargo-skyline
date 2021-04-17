pub fn get_plugins_path(title_id: &str) -> String {
    format!("/atmosphere/contents/{}/romfs/skyline/plugins", title_id)
}

pub fn get_plugin_path(title_id: &str, plugin_name: &str, user_path: Option<String>) -> String {
    if let Some(user_path) = user_path {
        if user_path.starts_with("/") {
            format!("/atmosphere/contents/{}/romfs{}", title_id, user_path)
        } else {
            format!("/atmosphere/contents/{}/romfs/{}", title_id, user_path)
        }
    } else {
        format!("/atmosphere/contents/{}/romfs/skyline/plugins/{}", title_id, plugin_name)
    }
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
