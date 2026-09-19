/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::BTreeMap;

pub type TPlatform = crate::grpc::Platform;
pub type TProperty = crate::grpc::Property;

#[derive(Clone, Default)]
pub struct ActionHistoryInfo {
    pub action_key: String,
    pub disable_retry_on_oom: bool,
    pub _dot_dot: (),
}

#[derive(Clone, Default)]
pub struct BuckInfo {
    pub build_id: String,
    pub version: String,
    pub _dot_dot: (),
}

#[derive(Clone, Default)]
pub struct TClientContextMetadata {
    pub attributes: BTreeMap<String, String>,
    pub _dot_dot: (),
}

/// What the action is, for REAPI `RequestMetadata`.
///
/// REv2 defines `target_id`, `action_mnemonic` and `configuration_id`, and the
/// OSS client has always sent them empty, so a server can attribute work to an
/// invocation but not to a target or a rule. Buck2 knows all three by the time
/// it talks to RE; they just had nowhere to travel.
#[derive(Clone, Default)]
pub struct ReRequestIdentity {
    /// `cell//package:name` of the configured target the action belongs to.
    pub target_id: String,
    /// The action's category, e.g. `cxx_compile`. A bounded set, so it is safe
    /// for a server to use as a metric label.
    pub action_mnemonic: String,
    /// Full name of the configuration the target is configured against.
    pub configuration_id: String,
    pub _dot_dot: (),
}

#[derive(Clone, Default)]
pub struct RemoteExecutionMetadata {
    pub action_history_info: Option<ActionHistoryInfo>,
    /// Absent for requests not tied to a single action, such as blob uploads
    /// done ahead of execution or a capabilities call.
    pub action_identity: Option<ReRequestIdentity>,
    pub buck_info: Option<BuckInfo>,
    pub platform: Option<TPlatform>,
    pub use_case_id: String,
    pub do_not_cache: bool,
    pub respect_file_symlinks: Option<bool>,
    pub client_context: Option<TClientContextMetadata>,
    pub disable_cancel_on_disconnect: bool,
    pub _dot_dot: (),
}
