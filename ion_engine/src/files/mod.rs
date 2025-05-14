use std::path::PathBuf;
use std::{fs, io};

use ion_common::{Map, log_info, log_warn};

use crate::core::Constants;
use crate::files::file_helpers::{list_dirs, list_files};
use crate::util::config::{Config, ConfigParseError, config_from_string, config_to_string};

pub mod file_helpers;
pub mod file_paths;

/// Cross-platform file system abstraction for the game engine.
///
/// The `Files` struct provides a unified interface for file operations that work on both native and wasm.
///
/// ## Platform Differences
///
/// - **Native platforms**: Uses the actual file system with platform-specific directories
/// - **WASM/Browser**: Uses browser local storage for configs and IndexedDB for save games
pub struct Files {
    app_name: String,
}

impl Files {
    pub fn new(constants: &Constants) -> Self {
        if cfg!(not(target_arch = "wasm32")) {
            fs::create_dir_all(file_paths::save_dir(constants.app_name, None))
                .unwrap_or_else(|_| panic!("Dir create error {:?}", file_paths::save_dir(constants.app_name, None)));
            fs::create_dir_all(file_paths::config_dir(constants.app_name))
                .unwrap_or_else(|_| panic!("Dir create error {:?}", file_paths::config_dir(constants.app_name)));
            fs::create_dir_all(file_paths::log_dir(constants.app_name))
                .unwrap_or_else(|_| panic!("Dir create error {:?}", file_paths::log_dir(constants.app_name)));
        }

        Self {
            app_name: constants.app_name.to_string(),
        }
    }

    /// Deletes all application data including configs, saves, and logs.
    /// ⚠️ **WARNING**: This will delete absolutely everything.
    pub fn delete_all_data(&self) -> Result<(), io::Error> {
        log_warn!("Deleting all application data");

        #[cfg(target_arch = "wasm32")]
        {
            // Delete all localStorage data
            for key in file_helpers::list_local_storage()? {
                file_helpers::delete_local_storage(&key)?;
            }
            // Delete all IndexedDB saves
            file_helpers::clear_store_indexeddb(&self.app_name, "saves")?;
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            fs::remove_dir_all(file_paths::game_data_dir(&self.app_name))
        }
    }

    /// Imports a configuration file from storage.
    pub fn import_config<T: Config>(&self, config_name: &str) -> Result<T, ConfigParseError> {
        log_info!("Importing config '{}'", config_name);
        let encoded = if cfg!(target_arch = "wasm32") {
            let storage_key = format!("{}_{}_config", self.app_name, config_name);
            file_helpers::read_local_storage(&storage_key)
                .map_err(|_| ConfigParseError::MissingData(format!("Missing config '{}'", config_name)))?
        } else {
            let config_dir = file_paths::config_dir(&self.app_name);
            let config_file = config_dir.join(config_name.replace(".conf", "").to_string() + ".conf");
            fs::read_to_string(config_file.clone())
                .map_err(|_| ConfigParseError::MissingData(format!("Missing file '{:?}'", config_file)))?
        };

        config_from_string(&encoded)
    }

    /// Exports a configuration file to storage.
    pub fn export_config(&self, config_name: &str, config: &dyn Config) -> io::Result<()> {
        log_info!("Exporting config '{}'", config_name);
        let encoded = config_to_string(config);

        if cfg!(target_arch = "wasm32") {
            let storage_key = format!("{}_{}_config", self.app_name, config_name);
            file_helpers::write_local_storage(&storage_key, &encoded)
        } else {
            let config_dir = file_paths::config_dir(&self.app_name);
            let config_file = config_dir.join(config_name.replace(".conf", "").to_string() + ".conf");
            fs::write(config_file, encoded)
        }
    }

    /// Deletes a specific configuration from storage.
    pub fn delete_config(&self, config_name: &str) -> Result<(), io::Error> {
        log_warn!("Deleting config '{}'", config_name);
        if cfg!(target_arch = "wasm32") {
            let storage_key = format!("{}_{}_config", self.app_name, config_name);
            file_helpers::delete_local_storage(&storage_key)
        } else {
            let config_dir = file_paths::config_dir(&self.app_name);
            let config_file = config_dir.join(config_name.replace(".conf", "").to_string() + ".conf");
            fs::remove_file(config_file)
        }
    }

    /// Deletes all configuration files from storage.
    /// ⚠️ **WARNING**: This will delete ALL configuration data!
    pub fn delete_all_configs(&self) -> Result<(), io::Error> {
        log_warn!("Deleting all configs");
        if cfg!(target_arch = "wasm32") {
            for key in file_helpers::list_local_storage()? {
                if key.ends_with("_config") {
                    file_helpers::delete_local_storage(&key)?;
                }
            }
            Ok(())
        } else {
            fs::remove_dir_all(file_paths::config_dir(&self.app_name))
        }
    }

    /// Exports save game data to storage.
    /// - **Native platforms**: Uses the file system with backup and restore functionality
    /// - **WASM/Browser**: Uses IndexedDB for persistent storage
    pub fn export_save(&self, name: &str, files: Vec<(String, Vec<u8>)>) -> Result<(), io::Error> {
        log_info!("Exporting save '{}'", name);

        #[cfg(target_arch = "wasm32")]
        {
            // Convert Vec to Map for IndexedDB storage
            let files_map: Map<String, Vec<u8>> = files.into_iter().collect();
            let js_object = file_helpers::files_map_to_js_object(&files_map);
            file_helpers::write_indexeddb(&self.app_name, "saves", name, &js_object)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let save_folder_path = file_paths::save_dir(&self.app_name, Some(name));
            let save_folder_backup_path =
                file_paths::save_dir(&self.app_name, Some(format!("{}_backup", name).as_str()));
            let prev_save_exists = save_folder_path.is_dir();

            // Backup previous save
            if prev_save_exists {
                fs::rename(&save_folder_path, &save_folder_backup_path)?;
            }

            fs::create_dir_all(&save_folder_path)?;
            let save_result: Result<(), io::Error> = {
                for (file_name, file_content) in files {
                    let file_path = save_folder_path.clone().join(PathBuf::from(file_name));
                    fs::write(save_folder_path.join(file_path), file_content)?;
                }
                Ok(())
            };

            if save_result.is_err() {
                // Error while writing save, restore backup
                fs::rename(&save_folder_backup_path, &save_folder_path)?;
                save_result
            } else {
                if prev_save_exists {
                    fs::remove_dir_all(save_folder_backup_path)?;
                }
                Ok(())
            }
        }
    }

    /// Imports save game data from storage.
    /// - **Native platforms**: Reads from the file system
    /// - **WASM/Browser**: Reads from IndexedDB
    pub fn import_save(&self, save_name: &str) -> Result<Map<String, Vec<u8>>, io::Error> {
        log_info!("Importing save '{}'", save_name);

        #[cfg(target_arch = "wasm32")]
        {
            let js_value = file_helpers::read_indexeddb(&self.app_name, "saves", save_name)?;
            file_helpers::js_object_to_files_map(&js_value)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let save_folder_path = file_paths::save_dir(&self.app_name, Some(save_name));
            let mut save_files: Map<String, Vec<u8>> = Map::default();

            for (file_path, _) in list_files(&save_folder_path, None)? {
                let file_content = fs::read(&file_path)?;
                let file_name = file_path
                    .file_name()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .map_err(|err| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Save file {:?} was not valid: {:?}", file_path, err),
                        )
                    })?;

                save_files.insert(file_name, file_content);
            }

            Ok(save_files)
        }
    }

    /// Deletes a save game from storage.
    /// - **Native platforms**: Removes from the file system
    /// - **WASM/Browser**: Removes from IndexedDB
    pub fn delete_save(&self, save_name: &str) -> Result<(), io::Error> {
        log_warn!("Deleting save '{}'", save_name);

        #[cfg(target_arch = "wasm32")]
        {
            file_helpers::delete_indexeddb(&self.app_name, "saves", save_name)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let save_folder_path = file_paths::save_dir(&self.app_name, Some(save_name));
            fs::remove_dir_all(save_folder_path)
        }
    }

    /// Lists all available save games.
    /// - **Native platforms**: Lists directories in the saves folder
    /// - **WASM/Browser**: Lists saves stored in IndexedDB
    pub fn list_saves(&self) -> Result<Vec<String>, io::Error> {
        #[cfg(target_arch = "wasm32")]
        {
            file_helpers::list_keys_indexeddb(&self.app_name, "saves")
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let paths = list_dirs(&file_paths::save_dir(&self.app_name, None))?;
            Ok(paths
                .into_iter()
                .map(|path| {
                    path.file_name()
                        .unwrap()
                        .to_os_string()
                        .into_string()
                        .expect("Save file names must be valid unicode")
                })
                .collect())
        }
    }
}

// ---------------------------------------------------------- //
// -------------------------- Tests ------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Constants;
    use crate::util::config::Config;
    use derive_engine::Config;

    // Mock config for testing
    #[derive(Debug, Clone, PartialEq, Config)]
    struct TestConfig {
        value: i32,
        name: String,
    }

    // Guard struct that ensures cleanup happens even on panic
    struct TestFilesGuard {
        files: Files,
    }

    impl TestFilesGuard {
        fn new(test_name: &str) -> Self {
            let app_name = format!("ion_test_{}", test_name);

            // Create a mock Constants with the correct structure and unique app name
            let constants = Constants {
                app_name: Box::leak(app_name.into_boxed_str()), // Convert to &'static str
                gfx: crate::core::GfxConstants {
                    asset_path: std::path::PathBuf::from("test_assets"),
                    camera_angle_deg: 45.0,
                    pixels_per_unit: 32.0,
                    height_units_total: 100.0,
                    height_scaled_zero: 0.5,
                },
                net: None,
            };

            let files = Files::new(&constants);
            Self { files }
        }

        fn files(&self) -> &Files {
            &self.files
        }
    }

    impl Drop for TestFilesGuard {
        fn drop(&mut self) {
            let _ = self.files.delete_all_data();
        }
    }

    #[test]
    fn test_new_creates_directories() {
        let _guard = TestFilesGuard::new("new_creates_directories");

        // Verify Files::new doesn't panic (native only behavior tested)
        if cfg!(not(target_arch = "wasm32")) {
            assert!(true, "Files::new completed without panicking");
        }
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_export_and_import_config() {
        let guard = TestFilesGuard::new("export_and_import_config");
        let files = guard.files();

        let test_config = TestConfig {
            value: 123,
            name: "test_config".to_string(),
        };

        // Export config
        let result = files.export_config("test", &test_config);
        assert!(result.is_ok(), "Failed to export config: {:?}", result);

        // Import config
        let imported: Result<TestConfig, _> = files.import_config("test");
        assert!(imported.is_ok(), "Failed to import config: {:?}", imported);

        let imported_config = imported.unwrap();
        assert_eq!(imported_config.value, test_config.value);
        assert_eq!(imported_config.name, test_config.name);
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_import_nonexistent_config() {
        let guard = TestFilesGuard::new("import_nonexistent_config");
        let files = guard.files();

        let result: Result<TestConfig, _> = files.import_config("nonexistent");
        assert!(result.is_err(), "Should fail to import nonexistent config");

        match result.unwrap_err() {
            ConfigParseError::MissingData(_) => (),
            other => panic!("Expected MissingData error, got: {:?}", other),
        }
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_config() {
        let guard = TestFilesGuard::new("delete_config");
        let files = guard.files();

        let test_config = TestConfig {
            value: 42,
            name: "test".to_string(),
        };

        // Export then delete config
        files.export_config("to_delete", &test_config).unwrap();

        let delete_result = files.delete_config("to_delete");
        assert!(delete_result.is_ok(), "Failed to delete config: {:?}", delete_result);

        // Verify config is gone
        let import_result: Result<TestConfig, _> = files.import_config("to_delete");
        assert!(import_result.is_err(), "Config should be deleted");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_nonexistent_config() {
        let guard = TestFilesGuard::new("delete_nonexistent_config");
        let files = guard.files();

        let result = files.delete_config("nonexistent");
        assert!(result.is_err(), "Should fail to delete nonexistent config");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_all_configs() {
        let guard = TestFilesGuard::new("delete_all_configs");
        let files = guard.files();

        // Create multiple configs
        let config1 = TestConfig {
            value: 1,
            name: "first".to_string(),
        };
        let config2 = TestConfig {
            value: 2,
            name: "second".to_string(),
        };

        files.export_config("config1", &config1).unwrap();
        files.export_config("config2", &config2).unwrap();

        // Delete all configs
        let result = files.delete_all_configs();
        assert!(result.is_ok(), "Failed to delete all configs: {:?}", result);

        // Verify configs are gone
        let import1: Result<TestConfig, _> = files.import_config("config1");
        let import2: Result<TestConfig, _> = files.import_config("config2");
        assert!(import1.is_err() && import2.is_err(), "All configs should be deleted");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_export_and_import_save() {
        let guard = TestFilesGuard::new("export_and_import_save");
        let files = guard.files();

        let save_files = vec![
            ("file1.dat".to_string(), vec![1, 2, 3, 4]),
            ("file2.txt".to_string(), b"Hello, World!".to_vec()),
            ("file3.bin".to_string(), vec![255, 0, 128]),
        ];

        // Export save
        let export_result = files.export_save("test_save", save_files.clone());
        assert!(export_result.is_ok(), "Failed to export save: {:?}", export_result);

        // Import save
        let import_result = files.import_save("test_save");
        assert!(import_result.is_ok(), "Failed to import save: {:?}", import_result);

        let imported_files = import_result.unwrap();

        // Verify all files were imported correctly
        for (filename, expected_content) in save_files {
            let imported_content = imported_files
                .get(&filename)
                .unwrap_or_else(|| panic!("Missing file: {}", filename));
            assert_eq!(imported_content, &expected_content, "Content mismatch for {}", filename);
        }
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_export_save_with_backup_and_restore() {
        let guard = TestFilesGuard::new("export_save_with_backup_and_restore");
        let files = guard.files();

        // Create initial save
        let initial_files = vec![("data.txt".to_string(), b"initial".to_vec())];
        files.export_save("backup_test", initial_files).unwrap();

        // Create new save (should backup the old one)
        let new_files = vec![("data.txt".to_string(), b"updated".to_vec())];
        let result = files.export_save("backup_test", new_files);
        assert!(result.is_ok(), "Failed to export save with backup: {:?}", result);

        // Verify the save was updated
        let imported = files.import_save("backup_test").unwrap();
        assert_eq!(imported.get("data.txt").unwrap(), &b"updated".to_vec());
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_import_nonexistent_save() {
        let guard = TestFilesGuard::new("import_nonexistent_save");
        let files = guard.files();

        let result = files.import_save("nonexistent_save");
        assert!(result.is_err(), "Should fail to import nonexistent save");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_save() {
        let guard = TestFilesGuard::new("delete_save");
        let files = guard.files();

        // Create a save to delete
        let save_files = vec![("test.dat".to_string(), vec![1, 2, 3])];
        files.export_save("to_delete", save_files).unwrap();

        // Delete the save
        let delete_result = files.delete_save("to_delete");
        assert!(delete_result.is_ok(), "Failed to delete save: {:?}", delete_result);

        // Verify save is gone
        let import_result = files.import_save("to_delete");
        assert!(import_result.is_err(), "Save should be deleted");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_nonexistent_save() {
        let guard = TestFilesGuard::new("delete_nonexistent_save");
        let files = guard.files();

        let result = files.delete_save("nonexistent_save");
        assert!(result.is_err(), "Should fail to delete nonexistent save");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_list_saves() {
        let guard = TestFilesGuard::new("list_saves");
        let files = guard.files();

        // Create multiple saves
        let save1 = vec![("data1.txt".to_string(), b"save1".to_vec())];
        let save2 = vec![("data2.txt".to_string(), b"save2".to_vec())];
        let save3 = vec![("data3.txt".to_string(), b"save3".to_vec())];

        files.export_save("save_alpha", save1).unwrap();
        files.export_save("save_beta", save2).unwrap();
        files.export_save("save_gamma", save3).unwrap();

        // List saves
        let saves_result = files.list_saves();
        assert!(saves_result.is_ok(), "Failed to list saves: {:?}", saves_result);

        let mut saves = saves_result.unwrap();
        saves.sort(); // Sort for consistent comparison

        assert_eq!(saves.len(), 3);
        assert!(saves.contains(&"save_alpha".to_string()));
        assert!(saves.contains(&"save_beta".to_string()));
        assert!(saves.contains(&"save_gamma".to_string()));
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_list_saves_empty() {
        let guard = TestFilesGuard::new("list_saves_empty");
        let files = guard.files();

        let saves_result = files.list_saves();
        // Should succeed even with no saves
        assert!(saves_result.is_ok(), "Should be able to list empty saves directory");

        let saves = saves_result.unwrap();
        assert_eq!(saves.len(), 0, "Should have no saves initially");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_delete_all_data() {
        let guard = TestFilesGuard::new("delete_all_data");
        let files = guard.files();

        // Create some data
        let config = TestConfig {
            value: 42,
            name: "test".to_string(),
        };
        let save_files = vec![("test.dat".to_string(), vec![1, 2, 3])];

        files.export_config("test_config", &config).unwrap();
        files.export_save("test_save", save_files).unwrap();

        // Delete all data
        let result = files.delete_all_data();
        assert!(result.is_ok(), "Failed to delete all data: {:?}", result);

        // Verify everything is gone
        let config_result: Result<TestConfig, _> = files.import_config("test_config");
        let save_result = files.import_save("test_save");

        assert!(config_result.is_err(), "Config should be deleted");
        assert!(save_result.is_err(), "Save should be deleted");
        // Guard automatically cleans up when it goes out of scope (but delete_all_data already cleaned up)
    }

    #[test]
    fn test_config_name_normalization() {
        let guard = TestFilesGuard::new("config_name_normalization");
        let files = guard.files();

        let config = TestConfig {
            value: 42,
            name: "test".to_string(),
        };

        // Test that .conf extension is handled correctly
        if cfg!(not(target_arch = "wasm32")) {
            files.export_config("test.conf", &config).unwrap();
            let imported: TestConfig = files.import_config("test").unwrap();
            assert_eq!(imported.value, config.value);
        }
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_save_with_empty_files_list() {
        let guard = TestFilesGuard::new("save_with_empty_files_list");
        let files = guard.files();

        // Export save with no files
        let result = files.export_save("empty_save", vec![]);
        assert!(result.is_ok(), "Should be able to create save with no files");

        // Import should succeed but return empty map
        let imported = files.import_save("empty_save").unwrap();
        assert_eq!(imported.len(), 0, "Empty save should have no files");
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_save_with_large_files() {
        let guard = TestFilesGuard::new("save_with_large_files");
        let files = guard.files();

        // Create a large file (1MB)
        let large_content = vec![0u8; 1024 * 1024];
        let save_files = vec![("large_file.dat".to_string(), large_content.clone())];

        let export_result = files.export_save("large_save", save_files);
        assert!(export_result.is_ok(), "Should handle large files");

        let imported = files.import_save("large_save").unwrap();
        let imported_content = imported.get("large_file.dat").unwrap();
        assert_eq!(imported_content.len(), large_content.len());
        // Guard automatically cleans up when it goes out of scope
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_save_overwrites_correctly() {
        let guard = TestFilesGuard::new("save_overwrites_correctly");
        let files = guard.files();

        // Create initial save with multiple files
        let initial_files =
            vec![("file1.txt".to_string(), b"version1".to_vec()), ("file2.txt".to_string(), b"data2".to_vec())];
        files.export_save("overwrite_test", initial_files).unwrap();

        // Overwrite with different files
        let new_files =
            vec![("file1.txt".to_string(), b"version2".to_vec()), ("file3.txt".to_string(), b"data3".to_vec())];
        files.export_save("overwrite_test", new_files).unwrap();

        // Verify the save was completely replaced
        let imported = files.import_save("overwrite_test").unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported.get("file1.txt").unwrap(), &b"version2".to_vec());
        assert_eq!(imported.get("file3.txt").unwrap(), &b"data3".to_vec());
        assert!(imported.get("file2.txt").is_none(), "Old file should be gone");
        // Guard automatically cleans up when it goes out of scope
    }
}
