/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use buck2_core::package::source_path::SourcePathRef;
use buck2_core::plugins::PluginKind;
use buck2_core::plugins::PluginKindSet;
use buck2_core::provider::label::ConfiguredProvidersLabel;
use buck2_core::target::label::label::TargetLabel;

use crate::attrs::attr_type::query::ResolvedQueryLiterals;

pub trait ConfiguredAttrTraversal {
    fn dep(&mut self, dep: &ConfiguredProvidersLabel) -> anyhow::Result<()>;

    fn dep_with_plugins(
        &mut self,
        dep: &ConfiguredProvidersLabel,
        _plugins: &PluginKindSet,
    ) -> anyhow::Result<()> {
        // By default, just treat it as a dep. Most things don't care about the distinction.
        self.dep(dep)
    }

    fn exec_dep(&mut self, dep: &ConfiguredProvidersLabel) -> anyhow::Result<()> {
        // By default, just treat it as a dep. Most things don't care about the distinction.
        self.dep(dep)
    }

    fn toolchain_dep(&mut self, dep: &ConfiguredProvidersLabel) -> anyhow::Result<()> {
        // By default, just treat it as a dep. Most things don't care about the distinction.
        self.dep(dep)
    }

    fn configuration_dep(&mut self, _dep: &TargetLabel) -> anyhow::Result<()> {
        Ok(())
    }

    fn plugin_dep(&mut self, _dep: &TargetLabel, _kind: &PluginKind) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called for both `attrs.query(...)` and query macros like `$(query_targets ...)`.
    fn query(
        &mut self,
        _query: &str,
        _resolved_literals: &ResolvedQueryLiterals<ConfiguredProvidersLabel>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn input(&mut self, _path: SourcePathRef) -> anyhow::Result<()> {
        Ok(())
    }

    fn label(&mut self, _label: &ConfiguredProvidersLabel) -> anyhow::Result<()> {
        Ok(())
    }
}
