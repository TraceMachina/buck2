/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fmt::Debug;

pub trait CommandExecutionTarget: Send + Sync + Debug {
    fn re_action_key(&self) -> String;

    fn re_affinity_key(&self) -> String;

    fn as_proto_action_key(&self) -> buck2_data::ActionKey;

    fn as_proto_action_name(&self) -> buck2_data::ActionName;

    /// The configured target label this action contributes to, for REAPI
    /// `RequestMetadata.target_id`.
    ///
    /// Derived from `as_proto_action_key` rather than required of every
    /// implementation, because the value is already there: this is the same
    /// owner the action key carries, flattened to the `cell//package:name`
    /// string a REAPI server can group by. An owner shape with no meaningful
    /// label yields an empty string, which is what the field held before.
    fn re_target_id(&self) -> String {
        use buck2_data::action_key::Owner;

        fn label_of(configured: &buck2_data::ConfiguredTargetLabel) -> String {
            match &configured.label {
                Some(label) => format!("{}:{}", label.package, label.name),
                None => String::new(),
            }
        }

        match self.as_proto_action_key().owner {
            Some(Owner::TargetLabel(l))
            | Some(Owner::TestTargetLabel(l))
            | Some(Owner::LocalResourceSetup(l)) => label_of(&l),
            Some(Owner::AnonTarget(t)) => match &t.name {
                Some(name) => format!("{}:{}", name.package, name.name),
                None => String::new(),
            },
            Some(Owner::BxlKey(_)) | None => String::new(),
        }
    }

    /// The action's category, for REAPI `RequestMetadata.action_mnemonic`.
    /// This is the `cxx_compile`-style family name, not the per-action
    /// identifier, so it stays a bounded set suitable for grouping.
    fn re_action_mnemonic(&self) -> String {
        self.as_proto_action_name().category
    }

    /// The configuration the target is configured against, for REAPI
    /// `RequestMetadata.configuration_id`. Empty when the owner carries none.
    fn re_configuration_id(&self) -> String {
        use buck2_data::action_key::Owner;

        let configuration = match self.as_proto_action_key().owner {
            Some(Owner::TargetLabel(l))
            | Some(Owner::TestTargetLabel(l))
            | Some(Owner::LocalResourceSetup(l)) => l.configuration,
            Some(Owner::AnonTarget(t)) => t.execution_configuration,
            Some(Owner::BxlKey(_)) | None => None,
        };

        configuration.map(|c| c.full_name).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FakeTarget(buck2_data::ActionKey, buck2_data::ActionName);

    impl CommandExecutionTarget for FakeTarget {
        fn re_action_key(&self) -> String {
            String::new()
        }
        fn re_affinity_key(&self) -> String {
            String::new()
        }
        fn as_proto_action_key(&self) -> buck2_data::ActionKey {
            self.0.clone()
        }
        fn as_proto_action_name(&self) -> buck2_data::ActionName {
            self.1.clone()
        }
    }

    fn configured(package: &str, name: &str, cfg: &str) -> buck2_data::ConfiguredTargetLabel {
        buck2_data::ConfiguredTargetLabel {
            label: Some(buck2_data::TargetLabel {
                package: package.to_owned(),
                name: name.to_owned(),
            }),
            configuration: Some(buck2_data::Configuration {
                full_name: cfg.to_owned(),
            }),
            execution_configuration: None,
        }
    }

    fn target(owner: Option<buck2_data::action_key::Owner>, category: &str) -> FakeTarget {
        FakeTarget(
            buck2_data::ActionKey {
                owner,
                ..Default::default()
            },
            buck2_data::ActionName {
                category: category.to_owned(),
                identifier: "some-identifier".to_owned(),
            },
        )
    }

    #[test]
    fn identity_comes_from_the_action_key() {
        let t = target(
            Some(buck2_data::action_key::Owner::TargetLabel(configured(
                "root//src",
                "hello",
                "cfg:linux-x86_64#deadbeef",
            ))),
            "cxx_compile",
        );

        assert_eq!(t.re_target_id(), "root//src:hello");
        assert_eq!(
            t.re_action_mnemonic(),
            "cxx_compile",
            "the mnemonic is the category, not the per-action identifier",
        );
        assert_eq!(t.re_configuration_id(), "cfg:linux-x86_64#deadbeef");
    }

    #[test]
    fn test_and_local_resource_owners_resolve_the_same_way() {
        for owner in [
            buck2_data::action_key::Owner::TestTargetLabel(configured("root//t", "t", "cfg")),
            buck2_data::action_key::Owner::LocalResourceSetup(configured("root//t", "t", "cfg")),
        ] {
            let t = target(Some(owner), "test");
            assert_eq!(t.re_target_id(), "root//t:t");
            assert_eq!(t.re_configuration_id(), "cfg");
        }
    }

    /// An owner shape with no meaningful label must yield the empty string the
    /// field held before, never a panic or a placeholder a server would group
    /// real work under.
    #[test]
    fn owners_without_a_label_stay_empty() {
        let bxl = target(
            Some(buck2_data::action_key::Owner::BxlKey(
                buck2_data::BxlFunctionKey::default(),
            )),
            "bxl",
        );
        assert_eq!(bxl.re_target_id(), "");
        assert_eq!(bxl.re_configuration_id(), "");

        let none = target(None, "");
        assert_eq!(none.re_target_id(), "");
        assert_eq!(none.re_configuration_id(), "");
    }
}
