/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This modules contains common options that are shared between different commands.
//! They are shared by composition together with flattening of the options.
//!
//! For example, to adopt config options, add the following field to the
//! command definition:
//!
//! ```ignore
//! #[derive(Debug, clap::Parser)]
//! struct MyCommand {
//!    #[clap(flatten)]
//!    config_opts: CommonConfigOptions,
//!    ...
//! }
//! ```

pub mod ui;

use std::path::Path;
use std::str::FromStr;

use buck2_cli_proto::common_build_options::ExecutionStrategy;
use buck2_cli_proto::config_override::ConfigType;
use buck2_cli_proto::ConfigOverride;
use buck2_cli_proto::TargetCfg;
use buck2_core::fs::fs_util;
use clap::ArgGroup;
use dupe::Dupe;
use gazebo::prelude::*;
use tracing::warn;

use crate::common::ui::CommonConsoleOptions;
use crate::path_arg::PathArg;

pub const EVENT_LOG: &str = "--event-log";
pub const NO_EVENT_LOG: &str = "--no-event-log";

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Dupe,
    Copy,
    clap::ArgEnum
)]
#[clap(rename_all = "lower")]
pub enum HostPlatformOverride {
    Default,
    Linux,
    MacOs,
    Windows,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Dupe,
    Copy,
    clap::ArgEnum
)]
#[clap(rename_all = "lower")]
pub enum HostArchOverride {
    Default,
    AArch64,
    X86_64,
}

/// Defines options related to commands that involves a streaming daemon command.
#[derive(Debug, clap::Parser, serde::Serialize, serde::Deserialize, Default)]
pub struct CommonDaemonCommandOptions {
    /// Write events to this log file
    #[clap(value_name = "PATH", long = EVENT_LOG)]
    pub event_log: Option<PathArg>,

    /// Do not write any event logs. Overrides --event-log. Used from `replay` to avoid recursive logging
    #[clap(long = NO_EVENT_LOG, hidden = true)]
    pub no_event_log: bool,

    /// Write command invocation id into this file.
    #[clap(long, value_name = "PATH")]
    pub(crate) write_build_id: Option<PathArg>,

    /// Write the invocation record (as JSON) to this path. No guarantees whatsoever are made
    /// regarding the stability of the format.
    #[clap(long, value_name = "PATH")]
    pub(crate) unstable_write_invocation_record: Option<PathArg>,
}

impl CommonDaemonCommandOptions {
    pub fn default_ref() -> &'static Self {
        static DEFAULT: CommonDaemonCommandOptions = CommonDaemonCommandOptions {
            event_log: None,
            no_event_log: false,
            write_build_id: None,
            unstable_write_invocation_record: None,
        };
        &DEFAULT
    }
}

/// Defines options for config and configuration related things. Any command that involves the build graph should include these options.
#[derive(Debug, clap::Parser, serde::Serialize, serde::Deserialize, Default)]
pub struct CommonBuildConfigurationOptions {
    #[clap(
        value_name = "SECTION.OPTION=VALUE",
        long = "config",
        short = 'c',
        help = "List of config options",
        // Needs to be explicitly set, otherwise will treat `-c a b c` -> [a, b, c]
        // rather than [a] and other positional arguments `b c`.
        number_of_values = 1
    )]
    pub config_values: Vec<String>,

    #[clap(
        value_name = "PATH",
        long = "config-file",
        help = "List of config file paths",
        number_of_values = 1
    )]
    pub config_files: Vec<String>,

    #[clap(
        long = "target-platforms",
        help = "Configuration target (one) to use to configure targets",
        number_of_values = 1,
        value_name = "PLATFORM"
    )]
    pub target_platforms: Option<String>,

    #[clap(
        value_name = "VALUE",
        long = "modifier",
        use_value_delimiter = true,
        value_delimiter=',',
        short = 'm',
        help = "A configuration modifier to configure all targets on the command line. This may be a constraint value target.",
        // Needs to be explicitly set, otherwise will treat `-c a b c` -> [a, b, c]
        // rather than [a] and other positional arguments `b c`.
        number_of_values = 1
    )]
    pub cli_modifiers: Vec<String>,

    #[clap(long, ignore_case = true, value_name = "HOST", arg_enum)]
    fake_host: Option<HostPlatformOverride>,

    #[clap(long, ignore_case = true, value_name = "ARCH", arg_enum)]
    fake_arch: Option<HostArchOverride>,

    /// Value must be formatted as: version-build (e.g., 14.3.0-14C18 or 14.1-14B47b)
    #[clap(long, value_name = "VERSION-BUILD")]
    fake_xcode_version: Option<String>,

    /// Disable runtime type checking in Starlark interpreter.
    ///
    /// This option is not stable, and can be used only locally
    /// to diagnose evaluation performance problems.
    #[clap(long)]
    pub disable_starlark_types: bool,

    /// Typecheck bzl and bxl files during evaluation.
    #[clap(long, hidden(true))]
    pub unstable_typecheck: bool,

    /// Record or show target call stacks.
    ///
    /// Starlark call stacks will be included in duplicate targets error.
    ///
    /// If a command outputs targets (like `targets` command),
    /// starlark call stacks will be printed after the targets.
    #[clap(long = "stack")]
    pub target_call_stacks: bool,

    /// If there are targets with duplicate names in `BUCK` file,
    /// skip all the duplicates but the first one.
    /// This is a hack for TD. Do not use this option.
    #[clap(long)]
    pub(crate) skip_targets_with_duplicate_names: bool,

    /// Re-uses any `--config` values (inline or via modefiles) if there's
    /// a previous command, otherwise the flag is ignored.
    ///
    /// If there is a previous command and `--reuse-current-config` is set,
    /// then the old config is used, ignoring any overrides.
    ///
    /// If there is no previous command but the flag was set, then the flag is ignored,
    /// the command behaves as if the flag was not set at all.
    #[clap(long)]
    pub reuse_current_config: bool,

    /// Used for exiting a concurrent command when a different state is detected.
    #[clap(long)]
    pub exit_when_different_state: bool,
}

impl CommonBuildConfigurationOptions {
    /// Produces a single, ordered list of config overrides. A `ConfigOverride`
    /// represents either a file, passed via `--config-file`, or a config value,
    /// passed via `-c`/`--config`. The relative order of those are important,
    /// hence they're merged into a single list.
    pub fn config_overrides(
        &self,
        matches: &clap::ArgMatches,
    ) -> anyhow::Result<Vec<ConfigOverride>> {
        fn with_indices<'a, T>(
            collection: &'a [T],
            name: &str,
            matches: &'a clap::ArgMatches,
        ) -> impl Iterator<Item = (usize, &'a T)> + 'a {
            let indices = matches.indices_of(name);
            let indices = indices.unwrap_or_default();
            assert_eq!(
                indices.len(),
                collection.len(),
                "indices len is not equal to collection len for flag `{}`",
                name
            );
            indices.into_iter().zip(collection)
        }

        // Relative paths passed on the command line are relative to the cwd
        // of the client, not the daemon, so perform path canonicalisation here.
        fn resolve_config_file_argument(arg: &str) -> anyhow::Result<String> {
            if arg.contains("//") {
                // Cell-relative path resolution would be performed by the daemon
                return Ok(arg.to_owned());
            }

            let path = Path::new(arg);
            if path.is_absolute() {
                return Ok(arg.to_owned());
            }

            let abs_path = fs_util::canonicalize(path)?;
            Ok(abs_path.to_string_lossy().into_owned())
        }

        let config_values_args = with_indices(&self.config_values, "config-values", matches).map(
            |(index, config_value)| {
                (
                    index,
                    ConfigOverride {
                        config_override: config_value.clone(),
                        config_type: ConfigType::Value as i32,
                    },
                )
            },
        );

        let config_file_args = with_indices(&self.config_files, "config-files", matches)
            .map(|(index, unresolved_file)| {
                let resolved_file = resolve_config_file_argument(unresolved_file)?;
                Ok((
                    index,
                    ConfigOverride {
                        config_override: resolved_file,
                        config_type: ConfigType::File as i32,
                    },
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut ordered_merged_configs: Vec<(usize, ConfigOverride)> = config_file_args;
        ordered_merged_configs.extend(config_values_args);
        ordered_merged_configs.sort_by(|(lhs_index, _), (rhs_index, _)| lhs_index.cmp(rhs_index));

        Ok(ordered_merged_configs.into_map(|(_, config_arg)| config_arg))
    }

    pub fn host_platform_override(&self) -> HostPlatformOverride {
        match &self.fake_host {
            Some(v) => *v,
            None => HostPlatformOverride::Default,
        }
    }
    pub fn host_arch_override(&self) -> HostArchOverride {
        match &self.fake_arch {
            Some(v) => *v,
            None => HostArchOverride::Default,
        }
    }
    pub fn host_xcode_version_override(&self) -> Option<String> {
        self.fake_xcode_version.to_owned()
    }

    pub fn target_cfg(&self) -> TargetCfg {
        TargetCfg {
            target_platform: self.target_platforms.clone().unwrap_or_default(),
            cli_modifiers: self.cli_modifiers.clone(),
        }
    }

    pub fn default_ref() -> &'static Self {
        static DEFAULT: CommonBuildConfigurationOptions = CommonBuildConfigurationOptions {
            config_values: vec![],
            config_files: vec![],
            cli_modifiers: vec![],
            target_platforms: None,
            fake_host: None,
            fake_arch: None,
            fake_xcode_version: None,
            disable_starlark_types: false,
            unstable_typecheck: false,
            target_call_stacks: false,
            skip_targets_with_duplicate_names: false,
            reuse_current_config: false,
            exit_when_different_state: false,
        };
        &DEFAULT
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BuildReportOption {
    /// Fill out the failures in build report as it was done by default in buck1.
    fill_out_failures: bool,

    /// Include package relative paths in the output.
    include_package_project_relative_paths: bool,
}

impl FromStr for BuildReportOption {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fill_out_failures = false;
        let mut include_package_project_relative_paths = false;

        if s.to_lowercase() == "fill-out-failures" {
            fill_out_failures = true;
        } else if s.to_lowercase() == "package-project-relative-paths" {
            include_package_project_relative_paths = true;
        } else {
            warn!(
                "Incorrect syntax for build report option. Got: `{}` but expected one of `fill-out-failures, package-project-relative-paths`",
                s.to_owned()
            )
        }
        Ok(BuildReportOption {
            fill_out_failures,
            include_package_project_relative_paths,
        })
    }
}

/// Defines common options for build-like commands (build, test, install).
#[allow(rustdoc::invalid_html_tags)]
#[derive(Debug, clap::Parser, serde::Serialize, serde::Deserialize)]
pub struct CommonBuildOptions {
    /// Print a build report
    ///
    /// --build-report=- will print the build report to stdout
    /// --build-report=<filepath> will write the build report to the file
    #[clap(long = "build-report", value_name = "PATH")]
    build_report: Option<String>,

    /// Comma separated list of build report options.
    ///
    /// The following options are supported:
    ///
    /// `fill-out-failures`:
    /// fill out failures the same way Buck1 would.
    ///
    /// `package-project-relative-paths`:
    /// emit the project-relative path of packages for the targets that were built.
    #[clap(
        long = "build-report-options",
        requires = "build-report",
        value_delimiter = ','
    )]
    build_report_options: Vec<BuildReportOption>,

    /// Deprecated. Use --build-report=-
    // TODO(cjhopman): this is probably only used by the e2e framework. remove it from there
    #[clap(long = "print-build-report", hidden = true)]
    print_build_report: bool,

    /// Number of threads to use during execution (default is # cores)
    // TODO(cjhopman): This only limits the threads used for action execution and it doesn't work correctly with concurrent commands.
    #[clap(short = 'j', long = "num-threads", value_name = "THREADS")]
    pub num_threads: Option<u32>,

    /// Enable only local execution. Will reject actions that cannot execute locally.
    #[clap(long, group = "build_strategy", env = "BUCK_OFFLINE_BUILD")]
    local_only: bool,

    /// Enable only remote execution. Will reject actions that cannot execute remotely.
    #[clap(long, group = "build_strategy")]
    remote_only: bool,

    /// Enable hybrid execution. Will prefer executing actions that can execute locally on the
    /// local host.
    #[clap(long, group = "build_strategy")]
    prefer_local: bool,

    /// Enable hybrid execution. Will prefer executing actions that can execute remotely on RE and will avoid racing local and remote execution.
    #[clap(long, group = "build_strategy")]
    prefer_remote: bool,

    /// Experimental: Disable all execution.
    #[clap(long, group = "build_strategy")]
    unstable_no_execution: bool,

    /// Do not perform remote cache queries or cache writes. If remote execution is enabled, the RE
    /// service might still deduplicate actions, so for e.g. benchmarking, using a random isolation
    /// dir is preferred.
    #[clap(long, env = "BUCK_OFFLINE_BUILD")]
    no_remote_cache: bool,

    /// Could be used to enable the action cache writes on the RE worker when no_remote_cache is specified
    #[clap(long, requires("no-remote-cache"))]
    write_to_cache_anyway: bool,

    /// Process dep files when they are generated (i.e. after running a command that produces dep
    /// files), rather than when they are used (i.e. before re-running a command that previously
    /// produced dep files). Use this when debugging commands that produce dep files. Note that
    /// commands that previously produced dep files will not re-run: only dep files produced during
    /// this command will be eagerly loaded.
    #[clap(long)]
    eager_dep_files: bool,

    /// Uploads every action to the RE service, regardless of whether the action needs to execute on RE.
    ///
    /// This is useful when debugging builds and trying to inspect actions which executed remotely.
    /// It's possible that the action result is cached but the action itself has expired. In this case,
    /// downloading the action itself would fail. Enabling this option would unconditionally upload
    /// all actions, thus you will not hit any expiration issues.
    #[clap(long)]
    upload_all_actions: bool,

    /// If Buck hits an error, do as little work as possible before exiting.
    ///
    /// To illustrate the effect of this flag, consider an invocation of `build :foo :bar`. The
    /// default behavior of buck is to do enough work to get a result for the builds of each of
    /// `:foo` and `:bar`, and no more. This means that buck will continue to complete the build of
    /// `:bar` after the build of `:foo` has failed; however, once one dependency of `:foo` has
    /// failed, other dependencies will be cancelled unless they are needed by `:bar`.
    ///
    /// This flag changes the behavior of buck to not wait on `:bar` to complete once `:foo` has
    /// failed. Generally, this flag only has an effect on builds that specify multiple targets.
    ///
    /// `--keep-going` changes the behavior of buck to not only wait on `:bar` once one dependency
    /// of `:foo` has failed, but to additionally attempt to build other dependencies of `:foo` if
    /// possible.
    #[clap(long, group = "fail-when")]
    fail_fast: bool,

    /// If Buck hits an error, continue doing as much work as possible before exiting.
    ///
    /// See `--fail-fast` for more details.
    #[clap(long, group = "fail-when")]
    keep_going: bool,

    /// If target is missing, then skip building instead of throwing error.
    #[clap(long)]
    skip_missing_targets: bool,

    /// If target is incompatible with the specified configuration, skip building instead of throwing error.
    /// This does not apply to targets specified with glob patterns `/...` or `:`
    /// which are skipped unconditionally.
    #[clap(long)]
    skip_incompatible_targets: bool,

    /// Materializes inputs for failed actions which ran on RE
    #[clap(long)]
    materialize_failed_inputs: bool,
}

impl CommonBuildOptions {
    fn build_report(&self) -> (bool, String) {
        match (self.print_build_report, &self.build_report) {
            (false, None) => (false, "".to_owned()),
            (_, Some(path)) if path != "-" => (true, path.to_owned()),
            _ => (true, "".to_owned()),
        }
    }

    pub fn to_proto(&self) -> buck2_cli_proto::CommonBuildOptions {
        let (unstable_print_build_report, unstable_build_report_filename) = self.build_report();
        let unstable_include_failures_build_report = self
            .build_report_options
            .iter()
            .any(|option| option.fill_out_failures);
        let unstable_include_package_project_relative_paths = self
            .build_report_options
            .iter()
            .any(|option| option.include_package_project_relative_paths);
        let concurrency = self
            .num_threads
            .map(|num| buck2_cli_proto::Concurrency { concurrency: num });

        buck2_cli_proto::CommonBuildOptions {
            concurrency,
            execution_strategy: if self.local_only {
                ExecutionStrategy::LocalOnly as i32
            } else if self.remote_only {
                ExecutionStrategy::RemoteOnly as i32
            } else if self.prefer_local {
                ExecutionStrategy::HybridPreferLocal as i32
            } else if self.prefer_remote {
                ExecutionStrategy::HybridPreferRemote as i32
            } else if self.unstable_no_execution {
                ExecutionStrategy::NoExecution as i32
            } else {
                ExecutionStrategy::Default as i32
            },
            unstable_print_build_report,
            unstable_build_report_filename,
            eager_dep_files: self.eager_dep_files,
            upload_all_actions: self.upload_all_actions,
            skip_cache_read: self.no_remote_cache,
            skip_cache_write: self.no_remote_cache && !self.write_to_cache_anyway,
            fail_fast: self.fail_fast,
            keep_going: self.keep_going,
            skip_missing_targets: self.skip_missing_targets,
            skip_incompatible_targets: self.skip_incompatible_targets,
            materialize_failed_inputs: self.materialize_failed_inputs,
            unstable_include_failures_build_report,
            unstable_include_package_project_relative_paths,
        }
    }
}

/// Common options for commands like `build` or `query`.
/// Not all the commands have all the options.
#[derive(Debug, clap::Parser, serde::Serialize, serde::Deserialize, Default)]
pub struct CommonCommandOptions {
    /// Buckconfig and similar options.
    #[clap(flatten)]
    pub config_opts: CommonBuildConfigurationOptions,

    /// UI options.
    #[clap(flatten)]
    pub console_opts: CommonConsoleOptions,

    /// Event-log options.
    #[clap(flatten)]
    pub event_log_opts: CommonDaemonCommandOptions,
}

/// Show-output options shared by `build` and `targets`.
#[derive(Debug, clap::Parser)]
#[clap(group(
    // Make mutually exclusive. A command may have at most one of the flags in
    // the following group.
    ArgGroup::default().args(&[
        "show-output",
        "show-full-output",
        "show-simple-output",
        "show-full-simple-output",
        "show-json-output",
        "show-full-json-output",
    ])
))]
pub struct CommonOutputOptions {
    /// Print the path to the output for each of the rules relative to the project root
    #[clap(long)]
    pub show_output: bool,

    /// Print the absolute path to the output for each of the rules
    #[clap(long)]
    pub show_full_output: bool,

    /// Print only the path to the output for each of the rules relative to the project root
    #[clap(long)]
    pub show_simple_output: bool,

    /// Print only the absolute path to the output for each of the rules
    #[clap(long)]
    pub show_full_simple_output: bool,

    /// Print the output paths relative to the project root, in JSON format
    #[clap(long)]
    pub show_json_output: bool,

    /// Print the output absolute paths, in JSON format
    #[clap(long)]
    pub show_full_json_output: bool,
}

#[derive(Debug, PartialEq)]
pub enum PrintOutputsFormat {
    Plain,
    Simple,
    Json,
}

impl CommonOutputOptions {
    pub fn format(&self) -> Option<PrintOutputsFormat> {
        if self.show_output || self.show_full_output {
            Some(PrintOutputsFormat::Plain)
        } else if self.show_simple_output || self.show_full_simple_output {
            Some(PrintOutputsFormat::Simple)
        } else if self.show_json_output || self.show_full_json_output {
            Some(PrintOutputsFormat::Json)
        } else {
            None
        }
    }

    pub fn is_full(&self) -> bool {
        self.show_full_output || self.show_full_simple_output || self.show_full_json_output
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use clap::Parser;

    use super::*;

    fn parse(args: &[&str]) -> anyhow::Result<CommonBuildConfigurationOptions> {
        Ok(CommonBuildConfigurationOptions::from_iter_safe(
            std::iter::once("program").chain(args.iter().copied()),
        )?)
    }

    #[test]
    fn short_opt_multiple() -> anyhow::Result<()> {
        let opts = parse(&["-m", "value1", "-m", "value2"])?;

        assert_eq!(opts.cli_modifiers, vec!["value1", "value2"]);

        Ok(())
    }

    #[test]
    fn short_opt_comma_separated() -> anyhow::Result<()> {
        let opts = parse(&["-m", "value1,value2"])?;

        assert_eq!(opts.cli_modifiers, vec!["value1", "value2"]);

        Ok(())
    }

    #[test]
    fn long_opt_multiple() -> anyhow::Result<()> {
        let opts = parse(&["--modifier", "value1", "--modifier", "value2"])?;

        assert_eq!(opts.cli_modifiers, vec!["value1", "value2"]);

        Ok(())
    }

    #[test]
    fn long_opt_comma_separated() -> anyhow::Result<()> {
        let opts = parse(&["--modifier", "value1,value2"])?;

        assert_eq!(opts.cli_modifiers, vec!["value1", "value2"]);

        Ok(())
    }

    #[test]
    fn comma_separated_and_multiple() -> anyhow::Result<()> {
        let opts = parse(&["--modifier", "value1,value2", "--modifier", "value3"])?;

        assert_eq!(opts.cli_modifiers, vec!["value1", "value2", "value3"]);

        Ok(())
    }

    #[test]
    fn space_separated_fails() -> anyhow::Result<()> {
        assert_matches!(parse(&["-m", "value1", "value2"]), Err(..));

        Ok(())
    }
}
