use std::ffi::OsStr;
use std::fs::File;
use std::fs::Metadata;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::{fs, io};

use crate::util::concurrency::block_on;
use ion_common::Map;
use ion_common::js_sys::Array;
use ion_common::js_sys::Object;
use ion_common::js_sys::Reflect;
use ion_common::js_sys::Uint8Array;
use ion_common::wasm_bindgen::JsCast;
use ion_common::wasm_bindgen::JsValue;
use ion_common::web_sys::IdbDatabase;
use ion_common::web_sys::IdbRequest;
use ion_common::web_sys::IdbTransactionMode;
use ion_common::web_sys::XmlHttpRequest;
use ion_common::web_sys::XmlHttpRequestResponseType;
use ion_common::web_sys::window;

/// List subdirectories in the given directory. Does not go recursively down the directory tree.
pub fn list_dirs(root_dir: &PathBuf) -> Result<Vec<PathBuf>, io::Error> {
    let mut found_dirs: Vec<PathBuf> = Vec::new();
    let dir_entries = fs::read_dir(root_dir)?;
    for entry in dir_entries {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let entry_path = entry.path();

        if entry_type.is_dir() {
            found_dirs.push(entry_path);
        }
    }
    Ok(found_dirs)
}

/// Recursively lists files in a folder.
/// Additionally, accepts a list of file types. Any file types not in the list are ignored.
pub fn list_files<T: AsRef<Path>>(
    root_dir: T,
    file_type_filter: Option<&[&OsStr]>,
) -> Result<Vec<(PathBuf, Metadata)>, io::Error> {
    let mut found_files: Vec<(PathBuf, Metadata)> = Vec::new();
    let dir_entries = fs::read_dir(root_dir)?;
    for entry in dir_entries {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let entry_path = entry.path();

        let metadata = entry
            .metadata()
            .expect("Metadata must be available for inspected files");

        if entry_type.is_dir() {
            found_files.append(&mut list_files(&entry_path, file_type_filter)?);
        } else if entry_type.is_file() {
            if let Some(filter) = file_type_filter {
                if filter.contains(&entry_path.extension().unwrap_or(OsStr::new(""))) {
                    found_files.push((entry_path, metadata));
                }
            } else {
                found_files.push((entry_path, metadata));
            }
        }
    }
    Ok(found_files)
}

/// Loads a resource file from platform-specific source.
/// On wasm it uses sync http request. As such, it cannot be used from the main thread.
pub fn load_resource<T: AsRef<Path>>(path: T) -> Result<Vec<u8>, io::Error> {
    if cfg!(not(target_arch = "wasm32")) {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    } else {
        let xhr = XmlHttpRequest::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create XHR: {:?}", e)))?;

        // Convert path to string URL
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Path contains invalid UTF-8 characters"))?;

        // Configure XHR
        xhr.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
        xhr.open_with_async("GET", path_str, false)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open XHR: {:?}", e)))?;

        // Send the request
        xhr.send()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to send XHR: {:?}", e)))?;

        // Check if the request was successful
        if xhr.status().unwrap_or(0) != 200 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("HTTP error: {}", xhr.status().unwrap_or(0)),
            ));
        }

        // Get the response as ArrayBuffer and convert to Vec<u8>
        let array_buffer = xhr
            .response()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get response: {:?}", e)))?;

        let uint8_array = Uint8Array::new(&array_buffer);
        let mut vec = vec![0; uint8_array.length() as usize];
        uint8_array.copy_to(&mut vec);

        Ok(vec)
    }
}

pub fn read_local_storage(key: &str) -> Result<String, io::Error> {
    if cfg!(not(target_arch = "wasm32")) {
        panic!("Reading local storage is supported only on wasm");
    }

    let window = window().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get window object"))?;
    let storage = window
        .local_storage()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get localStorage: {:?}", e)))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "localStorage is not available"))?;

    storage
        .get_item(key)
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to read from localStorage: {:?}", e),
            )
        })?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Key '{}' not found in localStorage", key),
            )
        })
}

pub fn write_local_storage(key: &str, content: &str) -> Result<(), io::Error> {
    if cfg!(not(target_arch = "wasm32")) {
        panic!("Writing local storage is supported only on wasm");
    }

    let window = window().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get window object"))?;
    let storage = window
        .local_storage()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get localStorage: {:?}", e)))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "localStorage is not available"))?;

    storage.set_item(key, content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to write to localStorage: {:?}", e),
        )
    })
}

pub fn delete_local_storage(key: &str) -> Result<(), io::Error> {
    if cfg!(not(target_arch = "wasm32")) {
        panic!("Deleting local storage is supported only on wasm");
    }

    let window = window().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get window object"))?;
    let storage = window
        .local_storage()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get localStorage: {:?}", e)))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "localStorage is not available"))?;

    storage.remove_item(key).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to delete from localStorage: {:?}", e),
        )
    })
}

pub fn list_local_storage() -> Result<Vec<String>, io::Error> {
    if cfg!(not(target_arch = "wasm32")) {
        panic!("Listing local storage is supported only on wasm");
    }

    let window = window().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get window object"))?;
    let storage = window
        .local_storage()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get localStorage: {:?}", e)))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "localStorage is not available"))?;

    let keys: Vec<Result<String, io::Error>> = (0..storage.length().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to get localStorage length: {:?}", e),
        )
    })?)
        .map(|i| {
            storage
                .key(i)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get localStorage key: {:?}", e)))
                .map(|value| value.expect("Local storage key must be available"))
        })
        .collect();

    Ok(keys.into_iter().filter_map(|k| k.ok()).collect())
}

//----------------------------------------------------------//
// ---------------------- IndexedDB ------------------------//
//----------------------------------------------------------//

/// A future that wraps an IdbRequest and resolves when the request completes
struct IdbRequestFuture {
    request: IdbRequest,
    waker: Option<Waker>,
}

impl IdbRequestFuture {
    fn new(request: IdbRequest) -> Self {
        let future = Self { request, waker: None };

        // Set up the event handlers
        let request_clone = future.request.clone();
        let closure = ion_common::wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: JsValue| {
            // Future will be polled again by the executor
        }) as Box<dyn FnMut(JsValue)>);

        request_clone.set_onsuccess(Some(closure.as_ref().unchecked_ref()));
        request_clone.set_onerror(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Keep the closure alive

        future
    }
}

impl Future for IdbRequestFuture {
    type Output = Result<JsValue, JsValue>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use ion_common::web_sys::IdbRequestReadyState;

        match self.request.ready_state() {
            IdbRequestReadyState::Done => {
                if let Some(error) = self.request.error().ok().flatten() {
                    Poll::Ready(Err(error.into()))
                } else {
                    Poll::Ready(Ok(self.request.result().unwrap_or(JsValue::NULL)))
                }
            }
            IdbRequestReadyState::Pending => {
                self.waker = Some(cx.waker().clone());
                Poll::Pending
            }
            _ => Poll::Pending,
        }
    }
}

/// Opens an IndexedDB database with the specified store
pub fn open_database(app_name: &str, store_name: &str) -> Result<IdbDatabase, io::Error> {
    let window = window().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get window object"))?;

    let idb_factory = window
        .indexed_db()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to get IndexedDB factory: {:?}", e),
            )
        })?
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "IndexedDB is not available"))?;

    let db_name = format!("{}_{}", app_name, store_name);
    let open_request = idb_factory
        .open_with_u32(&db_name, 1)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open database: {:?}", e)))?;

    // Set up onupgradeneeded handler
    let store_name_clone = store_name.to_string();
    let upgrade_closure = ion_common::wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(target) = Reflect::get(&event, &JsValue::from_str("target")) {
            if let Ok(result) = Reflect::get(&target, &JsValue::from_str("result")) {
                if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                    // Create object store (will only be called if database is being upgraded)
                    let _ = db.create_object_store(&store_name_clone);
                }
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    open_request.set_onupgradeneeded(Some(upgrade_closure.as_ref().unchecked_ref()));
    upgrade_closure.forget();

    let result = block_on(IdbRequestFuture::new(open_request.into()))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Database open failed: {:?}", e)))?;

    result
        .dyn_into::<IdbDatabase>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to cast to IdbDatabase: {:?}", e)))
}

/// Writes data to IndexedDB
pub fn write_indexeddb(app_name: &str, store_name: &str, key: &str, data: &JsValue) -> Result<(), io::Error> {
    let db = open_database(app_name, store_name)?;

    let transaction = db
        .transaction_with_str_and_mode(store_name, IdbTransactionMode::Readwrite)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create transaction: {:?}", e)))?;

    let store = transaction
        .object_store(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get object store: {:?}", e)))?;

    let put_request = store
        .put_with_key(data, &JsValue::from_str(key))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to put data: {:?}", e)))?;

    block_on(IdbRequestFuture::new(put_request))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Data storage failed: {:?}", e)))?;

    Ok(())
}

/// Reads data from IndexedDB
pub fn read_indexeddb(app_name: &str, store_name: &str, key: &str) -> Result<JsValue, io::Error> {
    let db = open_database(app_name, store_name)?;

    let transaction = db
        .transaction_with_str(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create transaction: {:?}", e)))?;

    let store = transaction
        .object_store(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get object store: {:?}", e)))?;

    let get_request = store
        .get(&JsValue::from_str(key))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get data: {:?}", e)))?;

    let result = block_on(IdbRequestFuture::new(get_request))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Data retrieval failed: {:?}", e)))?;

    if result.is_null() || result.is_undefined() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Key '{}' not found", key),
        ));
    }

    Ok(result)
}

/// Deletes data from IndexedDB
pub fn delete_indexeddb(app_name: &str, store_name: &str, key: &str) -> Result<(), io::Error> {
    let db = open_database(app_name, store_name)?;

    let transaction = db
        .transaction_with_str_and_mode(store_name, IdbTransactionMode::Readwrite)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create transaction: {:?}", e)))?;

    let store = transaction
        .object_store(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get object store: {:?}", e)))?;

    let delete_request = store
        .delete(&JsValue::from_str(key))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to delete data: {:?}", e)))?;

    block_on(IdbRequestFuture::new(delete_request))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Data deletion failed: {:?}", e)))?;

    Ok(())
}

/// Lists all keys in an IndexedDB store
pub fn list_keys_indexeddb(app_name: &str, store_name: &str) -> Result<Vec<String>, io::Error> {
    let db = open_database(app_name, store_name)?;

    let transaction = db
        .transaction_with_str(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create transaction: {:?}", e)))?;

    let store = transaction
        .object_store(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get object store: {:?}", e)))?;

    let request = store
        .get_all_keys()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get all keys: {:?}", e)))?;

    let result = block_on(IdbRequestFuture::new(request))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to list keys: {:?}", e)))?;

    let keys_array = result
        .dyn_into::<Array>()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Invalid keys format: {:?}", e)))?;

    let mut keys = Vec::new();
    for i in 0..keys_array.length() {
        let key = keys_array
            .get(i)
            .as_string()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid key format"))?;
        keys.push(key);
    }

    Ok(keys)
}

/// Clears all data from an IndexedDB store
pub fn clear_store_indexeddb(app_name: &str, store_name: &str) -> Result<(), io::Error> {
    let db = open_database(app_name, store_name)?;

    let transaction = db
        .transaction_with_str_and_mode(store_name, IdbTransactionMode::Readwrite)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create transaction: {:?}", e)))?;

    let store = transaction
        .object_store(store_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get object store: {:?}", e)))?;

    let clear_request = store
        .clear()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to clear store: {:?}", e)))?;

    block_on(IdbRequestFuture::new(clear_request))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to clear store: {:?}", e)))?;

    Ok(())
}

/// Helper function to convert a file map to a JavaScript object for storage
pub fn files_map_to_js_object(files: &Map<String, Vec<u8>>) -> JsValue {
    let save_object = Object::new();
    for (filename, content) in files {
        let uint8_array = Uint8Array::new_with_length(content.len() as u32);
        uint8_array.copy_from(content);
        let _ = Reflect::set(&save_object, &JsValue::from_str(filename), &uint8_array.into());
    }
    save_object.into()
}

/// Helper function to convert a JavaScript object back to a file map
pub fn js_object_to_files_map(js_value: &JsValue) -> Result<Map<String, Vec<u8>>, io::Error> {
    let mut files = Map::default();
    let save_object = js_value
        .dyn_ref::<Object>()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid data format - not an object"))?;

    let keys = Object::keys(save_object);
    for i in 0..keys.length() {
        let key = keys
            .get(i)
            .as_string()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Invalid file name in data"))?;

        let value = Reflect::get(save_object, &JsValue::from_str(&key))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get file data: {:?}", e)))?;

        let uint8_array = value
            .dyn_into::<Uint8Array>()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Invalid file data format: {:?}", e)))?;

        let mut content = vec![0u8; uint8_array.length() as usize];
        uint8_array.copy_to(&mut content);

        files.insert(key, content);
    }

    Ok(files)
}

// ---------------------------------------------------------- //
// ------------------------- Tests -------------------------- //
// ---------------------------------------------------------- //

#[cfg(test)]
mod tests {
    use crate::files::file_helpers::{list_dirs, list_files};
    use std::ffi::OsStr;
    use std::path::PathBuf;

    // A tmp folder that deletes itself when dropping the struct
    struct TmpFolder {
        path: PathBuf,
    }

    impl TmpFolder {
        pub fn create(path: &str) -> Self {
            let path = PathBuf::from(path);
            std::fs::create_dir_all(path.clone()).unwrap();
            Self { path }
        }

        pub fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }

    impl Drop for TmpFolder {
        fn drop(&mut self) {
            std::fs::remove_dir_all(self.path.clone()).unwrap();
        }
    }

    #[test]
    fn listing_files_in_directory_works() {
        let folder = TmpFolder::create("target/tmp/file_list_test");
        std::fs::write(folder.path().join("file_1.rs"), "").unwrap();
        std::fs::write(folder.path().join("file_2.rs"), "").unwrap();
        std::fs::create_dir(folder.path().join("sub_folder")).unwrap();
        std::fs::write(folder.path().join("sub_folder").join("file_3.rs"), "").unwrap();
        std::fs::write(folder.path().join("sub_folder").join("file_4.jpg"), "").unwrap();

        let list = list_files(&folder.path, Some(&[&OsStr::new("rs")])).unwrap();
        assert_eq!(list.len(), 3);
        dbg!(&list);
        assert!(
            list.iter()
                .any(|(a, _)| a == &PathBuf::from("target/tmp/file_list_test/file_1.rs"))
        );
        assert!(
            list.iter()
                .any(|(a, _)| a == &PathBuf::from("target/tmp/file_list_test/file_2.rs"))
        );
        assert!(
            list.iter()
                .any(|(a, _)| a == &PathBuf::from("target/tmp/file_list_test/sub_folder/file_3.rs"))
        );
    }

    #[test]
    fn listing_directories_in_directory_works() {
        let folder = TmpFolder::create("target/tmp/dir_list_test");
        std::fs::create_dir(folder.path().join("sub_folder")).unwrap();
        std::fs::create_dir(folder.path().join("sub_folder_2")).unwrap();

        let list = list_dirs(&folder.path).unwrap();
        assert_eq!(list.len(), 2);
        assert!(
            list.iter()
                .any(|a| a == &PathBuf::from("target/tmp/dir_list_test/sub_folder"))
        );
        assert!(
            list.iter()
                .any(|a| a == &PathBuf::from("target/tmp/dir_list_test/sub_folder_2"))
        );
    }
}
