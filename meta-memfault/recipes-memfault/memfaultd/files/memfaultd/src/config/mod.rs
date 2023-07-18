//
// Copyright (c) Memfault, Inc.
// See License.txt for details
use eyre::eyre;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    network::{NetworkClient, NetworkConfig},
    util::{DiskBacked, UnwrapOrDie, UpdateStatus},
};

use crate::util::disk_size::DiskSize;

pub use self::{
    config_file::{CoredumpCompression, JsonConfigs, MemfaultdConfig},
    device_config::{DeviceConfig, Resolution, Sampling},
    device_info::{DeviceInfo, DeviceInfoWarning},
};
use crate::mar::MarEntryBuilder;
use crate::mar::Metadata;
use eyre::{Context, Result};

mod config_file;
mod device_config;
mod device_info;

/// Container of the entire memfaultd configuration.
/// Implement `From<Config>` trait to initialize module specific configuration (see `NetworkConfig` for example).
pub struct Config {
    pub device_info: DeviceInfo,
    pub config_file: MemfaultdConfig,
    pub config_file_path: PathBuf,
    cached_device_config: Arc<RwLock<DiskBacked<DeviceConfig>>>,
}

const LOGS_SUBDIRECTORY: &str = "logs";
const MAR_STAGING_SUBDIRECTORY: &str = "mar";
const DEVICE_CONFIG_FILE: &str = "device_config.json";
const COREDUMP_RATE_LIMITER_FILENAME: &str = "coredump_rate_limit";

impl Config {
    pub const DEFAULT_CONFIG_PATH: &str = "/etc/memfaultd.conf";

    pub fn read_from_system(user_config: Option<&Path>) -> Result<Self> {
        // Select config file to read
        let config_file = user_config.unwrap_or_else(|| Path::new(Self::DEFAULT_CONFIG_PATH));

        let config = MemfaultdConfig::load(config_file)?;

        let (device_info, warnings) =
            DeviceInfo::load().wrap_err(eyre!("Unable to load device info"))?;
        warnings.iter().for_each(|w| eprintln!("{}", w));

        let device_config = DiskBacked::from_path(&Self::device_config_path_from_config(&config));

        Ok(Self {
            device_info,
            config_file: config,
            config_file_path: config_file.to_owned(),
            cached_device_config: Arc::new(RwLock::new(device_config)),
        })
    }

    pub fn refresh_device_config(&self, client: &impl NetworkClient) -> Result<UpdateStatus> {
        let response = client.fetch_device_config()?;

        // Let the server know that we have applied the new version if it still
        // believes we have an older one.
        let confirm_version = match response.data.completed {
            Some(v) if v == response.data.revision => None,
            _ => Some(response.data.revision),
        };

        // Always write the config to our cache.
        let new_config: DeviceConfig = response.into();
        let update_status = self
            .cached_device_config
            .write()
            .unwrap_or_die()
            .set(new_config)?;

        // After saving, create the device-config confirmation MAR entry
        if let Some(revision) = confirm_version {
            let mar_staging = self.mar_staging_path();
            MarEntryBuilder::new(&mar_staging)?
                .set_metadata(Metadata::new_device_config(revision))
                .save(&NetworkConfig::from(self))?;
        }
        Ok(update_status)
    }

    pub fn tmp_dir(&self) -> PathBuf {
        match self.config_file.tmp_dir {
            Some(ref tmp_dir) => tmp_dir.clone(),
            None => self.config_file.persist_dir.clone(),
        }
        .into()
    }

    pub fn tmp_dir_max_size(&self) -> DiskSize {
        DiskSize::new_capacity(self.config_file.tmp_dir_max_usage as u64)
    }

    pub fn tmp_dir_min_headroom(&self) -> DiskSize {
        DiskSize {
            bytes: self.config_file.tmp_dir_min_headroom as u64,
            inodes: self.config_file.tmp_dir_min_inodes as u64,
        }
    }

    pub fn coredump_rate_limiter_file_path(&self) -> PathBuf {
        self.tmp_dir().join(COREDUMP_RATE_LIMITER_FILENAME)
    }

    pub fn logs_path(&self) -> PathBuf {
        self.tmp_dir().join(LOGS_SUBDIRECTORY)
    }

    pub fn mar_staging_path(&self) -> PathBuf {
        self.tmp_dir().join(MAR_STAGING_SUBDIRECTORY)
    }

    fn device_config_path_from_config(config_file: &MemfaultdConfig) -> PathBuf {
        config_file.persist_dir.join(DEVICE_CONFIG_FILE)
    }
    pub fn device_config_path(&self) -> PathBuf {
        Self::device_config_path_from_config(&self.config_file)
    }

    pub fn sampling(&self) -> Sampling {
        if self.config_file.enable_dev_mode {
            Sampling::development()
        } else {
            self.device_config().sampling
        }
    }

    /// Returns the device_config at the time of the call. If the device_config is updated
    /// after this call, the returned value will not be updated.
    /// This can block for a small moment if another thread is currently updating the device_config.
    fn device_config(&self) -> DeviceConfig {
        self.cached_device_config
            .read()
            // If another thread crashed while holding the mutex we want to crash the program
            .unwrap_or_die()
            // If we were not able to load from local-cache then return the defaults.
            .get()
            .clone()
    }
}

#[cfg(test)]
impl Config {
    pub fn test_fixture() -> Self {
        Config {
            device_info: DeviceInfo::test_fixture(),
            config_file: MemfaultdConfig::test_fixture(),
            config_file_path: PathBuf::from("test_fixture.conf"),
            cached_device_config: Arc::new(RwLock::new(DiskBacked::from_path(&PathBuf::from(
                "test_fixture.json",
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::create_dir_all, path::PathBuf};

    use rstest::{fixture, rstest};

    use crate::{
        config::Config,
        mar::MarEntry,
        network::{
            DeviceConfigResponse, DeviceConfigResponseConfig, DeviceConfigResponseData,
            DeviceConfigResponseResolution, MockNetworkClient,
        },
        util::path::AbsolutePath,
    };

    #[test]
    fn tmp_dir_defaults_to_persist_dir() {
        let config = Config::test_fixture();

        assert_eq!(config.tmp_dir(), config.config_file.persist_dir);
    }

    #[test]
    fn tmp_folder_set() {
        let mut config = Config::test_fixture();
        let abs_path = PathBuf::from("/my/abs/path");
        config.config_file.tmp_dir = Some(AbsolutePath::try_from(abs_path.clone()).unwrap());

        assert_eq!(config.tmp_dir(), abs_path);
    }

    #[rstest]
    fn generate_mar_device_config_confirmation_when_needed(mut fixture: Fixture) {
        fixture
            .client
            .expect_fetch_device_config()
            .return_once(|| Ok(DEVICE_CONFIG_SAMPLE));
        fixture
            .config
            .refresh_device_config(&fixture.client)
            .unwrap();

        assert_eq!(fixture.count_mar_entries(), 1);
    }

    #[rstest]
    fn do_not_generate_mar_device_config_if_not_needed(mut fixture: Fixture) {
        let mut device_config = DEVICE_CONFIG_SAMPLE;
        device_config.data.completed = Some(device_config.data.revision);

        fixture
            .client
            .expect_fetch_device_config()
            .return_once(move || Ok(device_config));
        fixture
            .config
            .refresh_device_config(&fixture.client)
            .unwrap();

        assert_eq!(fixture.count_mar_entries(), 0);
    }

    struct Fixture {
        config: Config,
        _tmp_dir: tempfile::TempDir,
        client: MockNetworkClient,
    }

    #[fixture]
    fn fixture() -> Fixture {
        Fixture::new()
    }

    impl Fixture {
        fn new() -> Self {
            let tmp_dir = tempfile::tempdir().unwrap();
            let mut config = Config::test_fixture();
            config.config_file.persist_dir = tmp_dir.path().to_path_buf().try_into().unwrap();
            create_dir_all(config.mar_staging_path()).unwrap();
            Self {
                config,
                _tmp_dir: tmp_dir,
                client: MockNetworkClient::new(),
            }
        }

        fn count_mar_entries(self) -> usize {
            MarEntry::iterate_from_container(&self.config.mar_staging_path())
                .unwrap()
                .count()
        }
    }

    const DEVICE_CONFIG_SAMPLE: DeviceConfigResponse = DeviceConfigResponse {
        data: DeviceConfigResponseData {
            completed: None,
            revision: 42,
            config: DeviceConfigResponseConfig {
                memfault: crate::network::DeviceConfigResponseMemfault {
                    sampling: crate::network::DeviceConfigResponseSampling {
                        debugging_resolution: DeviceConfigResponseResolution::High,
                        logging_resolution: DeviceConfigResponseResolution::High,
                        monitoring_resolution: DeviceConfigResponseResolution::High,
                    },
                },
            },
        },
    };
}
