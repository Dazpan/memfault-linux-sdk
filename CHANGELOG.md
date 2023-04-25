# Memfault Linux SDK Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0] - 2023-04-25

### tldr

This release includes a number of changes that will require changes in your
project:

- **Edit your `bblayers.conf`** to stop using `meta-rust-bin` layer from the
  rust-embedded GitHub account and use the version provided in the
  `memfault-linux-sdk` repository.
- **Edit `memfault.conf`** to replace `data_dir` by `persist_dir` and carefully
  review `tmp_dir` (which defaults to `persist_dir`) and associated options to
  control maximum usage and minimum headroom. You will most likely need to set
  your own values.
- If you were calling `memfaultd --enable-data-collection` before, you need to
  replace it by `memfaultctl enable-data-collection` now.

### Added

- Memfaultd will now consider the amount of disk space and inodes remaining on
  disk when writing logs, storing coredumps and when cleaning the MAR staging
  area. See new options `tmp_dir_min_headroom_kib`, `tmp_dir_min_inodes` and
  `tmp_dir_max_usage_kib` in the [configuration file][reference-configuration].
- Logging is now rate limited on device (defaults to 500 lines per minute - see
  [`max_lines_per_minute][reference-logging]).
- We simplified the configuration options relative to data storage. Users are
  now expected to set a `persist_dir` option that must be persisted across
  reboots and a `tmp_dir` option that can be cleared on reboot (a temp
  filesystem in RAM). Refer to [Memfault Integration Guide -
  Storage][integration-storage] for more details.
- Option `logs.compression_level` to set the logs compression level.

### Changed

- Memfault Linux SDK now ships with a version of `meta-rust-bin` using a renamed
  Yocto class `cargo_bin`. This was required due to `meta-rust-bin` being
  incompatible with some poky packages. We will track the upstream bug and
  switch back to upstream `meta-rust-bin` when possible (see meta-rust-bin#135).
- `memfaultd` does not include the commands `enable-dev-mode` and
  `enable-data-collection` anymore (they were deprecated in 1.2.0.)
- We now consider logging to be ready for production use and have turned on
  `plugin_logging` by default.
- Some CMake improvements to build with older versions of GCC.
- Rewrote more `memfaultctl` commands to rust: `trigger-coredump`,
  `show-settings`, `sync`, `write-attributes`, `enable-dev-mode` and
  `enable-data-collection`.

### Removed

- Configuration options `logs.tmp_folder`, `mar.storage_max_usage_kib`,
  `coredump.storage_max_usage_kib` and `coredump.storage_min_headroom_kib` have
  been removed and are replaced by the new options listed above.
- `memfaultd --enable-data-collection` and `--enable-dev-mode` (as well as
  `--disable...`) have been removed.

### Fixed

- Bug causing coredump-handler to not capture coredumps in development mode.
- Bug causing coredump-handler to create a ratelimiter in the wrong place and
  fail the capture when it did not have permission to create the file.
- Fluent-bit connector will drop all logs when data collection is not enabled.
- Fluent-bit recommended configuration now includes a `Retry_Limit`.
- Wait until memfaultd is ready to write PID file.
- Fixed occasional error message `error sending on closed channel` on shutdown.
- Fix bug where `memfaultd` and `memfaultctl` would not properly report their
  version number.
- Show immediate error to the user when `memfaultctl write-attributes` is called
  but data collection is disabled.
- Fix build error when logging was disabled.

[reference-configuration]:
  https://docs.memfault.com/docs/linux/reference-memfaultd-configuration#top-level-etcmemfaultdconf-configuration
[reference-logging]:
  https://docs.memfault.com/docs/linux/reference-memfaultd-configuration#logs

## [1.3.2] - 2023-04-06

### Changed

- The Yocto layer meta-memfault does not depend on swupdate, collectd and
  fluent-bit anymore. Instead these dependencies are added by the memfaultd
  recipe and only when the corresponding plugins are enabled.

### Fixed

- Fix Yocto recipe to always enable network access during compilation and add
  `openssl` as a dependency.
- Updated architecture diagram to include fluent-bit

## [1.3.1] - 2023-03-22

### Added

- Add configuration in `meta-memfault-example` to run on Raspberry Pi 2/3/4.

### Changed

- Log files are now stored compressed on disk to reduce disk usage.
- To upload Memfault MAR entries (including logs), they are now streamed
  directly from disk without writing the MAR zip file to disk. This reduces disk
  I/O (flash wear) and means logs are only written once to disk which is
  optimal.
- Display server error text for Memfault API endpoints. This helps debug
  configuration issues.
- Validate the provided `device_id` and show an error if it will not be accepted
  by Memfault.
- Removed memfaultd dependency on libuboot. It was used to detect OTA reboots
  but we are now configuring swupdate to call `memfaultctl reboot --reason 3`
  after installing an upgrade.

### Fixed

- Fixed consistency of logfiles' Cid/NextCid which will help the Memfault
  dashboard identify discontinuity in the series of logs.
- Fixed the sleep duration displayed after a network error (memfaultd would
  announce sleeping for an hour but it would actually retry sooner).
- Fix a configuration problem where `collectd` logs would not be visible in the
  Memfault Dashboard (logs sent only to syslog are not captured by the default
  configuration - we are now configuring `collectd` to log to the standard
  output which is captured by `journald`).

## [1.3.0] - 2023-03-06

### Added

- Memfault SDK on Linux now supports Memfault archives (MAR), also used in our
  Android SDK. Going forward this is how all data will be stored on disk.
- A local TCP endpoint, compatible with fluent-bit tcp output plugin, is now
  available to capture logs. Logs are written to disk in MAR (Memfault ARchive)
  format and uploaded to Memfault when the device is online. **This feature is
  in technical preview stage and is disabled by default.** See [logging on
  linux][linux-logging] for more information.
- `meta-memfault-example` now includes fluent-bit to demonstrate how to collect
  logs.
- Memfault Linux SDK is now partially written in Rust. Our Yocto layer requires
  cargo and rust 1.65.0. We recommend [meta-rust-bin] from the rust-embedded
  project.
  - 🚧 `memfaultd` in the Linux SDK is currently a mix of C code and Rust.
    Please excuse the noise while we continue construction. 🚧
- Memfault agent can now be built on Linux and macOS systems (`cargo build`).

[meta-rust-bin]: https://github.com/rust-embedded/meta-rust-bin
[linux-logging]: https://docs.memfault.com/docs/linux/logging

### Changed

- `memfaultd` can now capture coredumps of itself.

### Fixed

- Fix bug where we restarted swupdate instead of swupdate.service. This removes
  a warning in the logs.
- Added link to the changelog in the release notes.
- Fix a bug where memfault would ignore SIGUSR1 signal while it was processing
  uploads.
- Fix a bug in the coredump capturing code that would cause a crash in case more
  than 16 warnings got emitted during the capture process. Thanks to
  [@attilaszia](https://github.com/attilaszia) for reporting this issue.

## [1.2.0] - 2022-12-26

### Added

- [memfaultctl] Added a new command `memfaultctl` to interact with `memfaultd`.
  - `memfaultctl trigger-coredump` to force a coredump generation and upload.
  - `memfaultctl request-metrics` to force `collectd` to flush metrics to
    Memfault.
  - `memfaultctl reboot` to save a reboot reason and restart the system.
  - `memfaultctl sync` to process `memfaultd` queue immediately.
  - `memfaultctl write-attributes` to push device attributes to Memfault.
  - 'Developer Mode` to reduce rate limits applied to coredumps during
    development.

### Changed

- Our Docker container now runs on Apple silicon without Rosetta emulation.
- Updated the `memfault-cli` package in the Docker image.
- Added "preferred versions" for `swupdate` and `collectd`.
- Coredumps are now compressed with gzip reducing storage and network usage.
- `memfaultd` is now built with `-g3`.

### Deprecated

- `memfaultd --(enable|disable)-dev-collection` and `memfaultctl -s` are now
  replaced by equivalent commands on `memfaultctl` and will be removed in a
  future version.

### Fixed

- `swupdate` would get in a bad state after reloading `memfaultd`. This is fixed
  by restarting both `swupdate` and `swupdate.socket` units.

## [1.1.0] - 2022-11-10

### Added

- [memfaultd] A new `last_reboot_reason_file` API has been added to enable
  extending the reboot reason determination subsystem. More information can be
  found in [the documentation of this feature][docs-reboots].
- [memfaultd] `memfaultd` will now take care of cleaning up `/sys/fs/pstore`
  after a reboot of the system (but only if the reboot reason tracking plugin,
  `plugin_reboot`, is enabled). Often, [systemd-pstore.service] is configured to
  carry out this task. This would conflict with `memfaultd` performing this
  task. Therefore, [systemd-pstore.service] is automatically excluded when
  including the `meta-memfault` layer. Note that `memfaultd` does not provide
  functionality (yet) to archive pstore files (like [systemd-pstore.service]
  can). If this is necessary for you, the work-around is to create a service
  that performs the archiving and runs before `memfaultd.service` starts up.

### Changed

- [memfaultd] When `memfaultd` would fail to determine the reason for a reboot,
  it would assume that "low power" was reason for the reboot. This makes little
  sense because there are many resets for which `memfaultd` is not able to
  determine a reason. This fallback is now changed to use "unspecified" in case
  the reason could not be determined (either from the built-in detection or
  externally, via the new `last_reboot_reason_file` API). Read the [new
  `last_reboot_reason_file` API][docs-reboots] for more information.
- Various improvements to the QEMU example integration:
  - It can now also be built for `qemuarm` (previously, only `qemuarm64` was
    working).
  - Linux pstore/ramoops subsystems are now correctly configured for the QEMU
    example integration, making it possible to test out the tracking of kernel
    panic reboot reasons using the QEMU device.
- [memfaultd] The unit test set up is now run on `x86_64` as well as `i386` to
  get coverage on a 64-bit architecture as wel as a 32-bit one.

### Fixed

- [memfaultd] Building the SDK on 32-bit systems would fail due to compilation
  errors. These are now fixed.
- [collectd] In the example, the statsd plugin would be listening on all network
  interfaces. This is narrowed to only listen on localhost (127.0.0.1).
- [memfaultd] Many improvements to reboot reason tracking:
  - Intermittently, a reboot would erroneously be attributed to "low power".
  - Kernel panics would show up in the application as "brown out reset".
  - Sometimes, multiple reboot events for a single Linux reboot would get
    emitted. The root causes have been found and fixed. Logic has been added
    that tracks the Linux `boot_id` to ensure that at most one reboot reason
    gets emitted per Linux boot.
  - When using the example integration, the reboot reason "firmware update"
    would not be detected after SWUpdate had installed an OTA update. This was
    caused by a mismatch of the `defconfig` file in the example integration and
    the version of SWUpdate that was being compiled. This is now corrected.
- [memfaultd] Fixed a bug in queue.c where an out-of-memory situation could lead
  to the queue's mutex not getting released.
- Improved the reliability of some of the E2E test scripts.

### Known Issues

- When `memfaultd --enable-data-collection` is run and data collection had not
  yet been enabled, it will regenerate the SWUpdate configuration and restart
  the `swupdate.service`. This restart can cause SWUpdate to get into a bad
  state and fail to install OTA updates. This is not a new issue and was already
  present in previous releases. We are investigating this issue. As a
  work-around, the device can be rebooted immediately after running
  `memfaultd --enable-data-collection`.
- The [systemd-pstore.service] gets disabled when including `meta-memfault`,
  even if `plugin_reboot` is disabled. As a work-around, if you need to keep
  [systemd-pstore.service], remove the `systemd_%.bbappend` file from the SDK.

[1.1.0]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.1.0-kirkstone
[systemd-pstore.service]:
  https://www.freedesktop.org/software/systemd/man/systemd-pstore.service.html
[docs-reboots]: https://mflt.io/linux-reboots

## [1.0.0] - 2022-09-28

### Added

- This release is the first one including support for collecting and uploading
  user-land coredumps to the Memfault platform. The coredump plugin is enabled
  by default. Alongside this SDK release, an accompanying [Memfault
  CLI][docs-cli] version 0.11.0 aids in uploading symbol files to Memfault from
  Yocto builds to facilitate making use of the new functionality. Uploading
  symbols is a necessary step in order to use Memfault for coredumps. [Read more
  about coredump support in the Memfault Linux SDK][docs-coredumps].

[docs-coredumps]: https://mflt.io/linux-coredumps
[docs-cli]: https://mflt.io/memfault-cli

### Changed

- Breaking changes in the format of `/etc/memfaultd.conf` (see [the updated
  reference][docs-reference-memfaultd-conf]):
  - The `collectd` top-level key was merged into the `collectd_plugin` top-level
    key. The fields previously in `collectd` that have been moved to
    `collectd_plugin` are:
    - `interval_seconds`
    - `non_memfaultd_chain`
    - `write_http_buffer_size_kib`
  - The `collectd_plugin.output_file` key has been replaced by two new keys:
    - `collectd_plugin.header_include_output_file`: the value of which should be
      included as the first statement in your `/etc/collectd.conf` file, and
    - `collectd_plugin.footer_include_output_file`: to be included as the last
      statement of your `/etc/collectd.conf` file.

[docs-reference-memfaultd-conf]:
  https://docs.memfault.com/docs/linux/reference-memfaultd-configuration/

### Fixed

- A misconfiguration bug whereby setting `collectd.interval_seconds` (now
  `collectd_plugin.interval_seconds`, see the "Changed" section of this release)
  would have no effect if our include file was at the bottom of
  `/etc/collectd.conf`. It happened due to the fact that collectd `Interval`
  statements are evaluated as they appear in source code (see [the author's
  statement][collectd-interval-eval]), only affecting the plugin statements that
  come after it.

[collectd-interval-eval]:
  https://github.com/collectd/collectd/issues/2444#issuecomment-331804766

### Known Issues

The server-side issue mentioned below has been resolved in the meantime.

~~Temporarily, our backend processing pipeline is unable to process coredumps
that link to shared objects in a specific style. This affects, in particular,
coredumps coming from devices on the Dunfell release of Yocto.~~

~~A backend fix has already been identified and should be released in the next
few business days. Once released, any previously collected coredumps that are
affected will be reprocessed server-side to address this issue. This will
**not** require any action from your team.~~

## [0.3.1] - 2022-09-05

### Added

- Support for Yocto version 3.1 (code name "Dunfell"). See the
  [`dunfell` branch](https://github.com/memfault/memfault-linux-sdk/tree/dunfell)
  of the repository.

### Changed

- The SDK repository no longer has a `main` branch. The variant of the SDK that
  supports Yocto 4.0 ("Kirkstone") can be found on the
  [branch named `kirkstone`](https://github.com/memfault/memfault-linux-sdk/tree/kirkstone).
  Likewise, the variant of the SDK that supports Yocto 3.1 ("Dunfell) can be
  found on
  [the branch called `dunfell`](https://github.com/memfault/memfault-linux-sdk/tree/dunfell).

## [0.3.0] - 2022-08-31

### Added

- Initial support for collecting metrics using [collectd]. Check out the
  [docs on Metrics for Linux](https://mflt.io/linux-metrics) for more
  information.

[collectd]: https://collectd.org/

## [0.2.0] - 2022-08-10

This is our first public release. Head over to [our Linux
documentation][docs-linux] for an introduction to the Memfault Linux SDK.

[docs-linux]: https://docs.memfault.com/docs/linux/introduction

### Added

- [memfaultd] Now implements exponential back-off for uploads. Requests
  originating from this exponential back-off system do not interfere with the
  regular upload interval.
- [memfaultd] Sets persisted flag to disable data collection and returns
  immediately: `memfaultd --disable-data-collection`.
- [memfaultd] The `builtin.json` configuration file now features a link to
  documentation for reference.
- Improved the top-level `README.md` with a feature and architecture overview.

### Fixed

- [memfaultd] The `--enable-data-collection` flag was not working reliably.
- [memfaultd] A parsing bug going through the output of `memfault-device-info`.

### Known Issues

During start-up of the `memfaultd` service, you may see a log line in the output
of `journalctl --unit memfaultd`:

```
memfaultd.service: Can't open PID file /run/memfaultd.pid (yet?) after start: Operation not permitted
```

This file is only used by `systemd` during service shut-down and its absence
during start-up does not affect the functioning of the daemon. A fix is planned
for a future release. See [this report on the Ubuntu `nginx`
package][nginx-pid-report] for a discussion on the topic.

[nginx-pid-report]: https://bugs.launchpad.net/ubuntu/+source/nginx/+bug/1581864

## [0.1.0] - 2022-07-27

### Added

- [memfaultd] Support reporting reboot reasons.
- [memfaultd] Support OTA updates via SWUpdate.
- A memfaultd layer for Yocto (meta-memfault).
- An example Yocto image using memfaultd and the features above
  (meta-memfault-example).

[0.1.0]: https://github.com/memfault/memfault-linux-sdk/releases/tag/0.1.0
[0.2.0]: https://github.com/memfault/memfault-linux-sdk/releases/tag/0.2.0
[0.3.0]: https://github.com/memfault/memfault-linux-sdk/releases/tag/0.3.0
[0.3.1]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/0.3.1-kirkstone
[1.0.0]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.0.0-kirkstone
[1.2.0]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.2.0-kirkstone
[1.3.0]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.3.0-kirkstone
[1.3.1]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.3.1-kirkstone
[1.3.2]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.3.2-kirkstone
[1.4.0]:
  https://github.com/memfault/memfault-linux-sdk/releases/tag/1.4.0-kirkstone
