# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under both the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree and the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree.

load("@prelude//rust:build.bzl", "crate_root")
load(
    "@prelude//rust:context.bzl",
    "DepCollectionContext",  # @unused Used as a type
)
load("@prelude//rust:link_info.bzl", "get_available_proc_macros", "resolve_rust_deps")

RustAnalyzerInfo = provider(
    fields = {
        # The root source for the rust target (typically lib.rs, main.rs), relative to the buck target file.
        "crate_root": str,
        # The list of rust deps needed for RustAnalyzer to function. Namely, this excludes things like
        # exec deps used as inputs to genrules and other non-rust dependencies.
        "rust_deps": list[Dependency],
    },
)

def _compute_rust_deps(
        ctx: AnalysisContext,
        dep_ctx: DepCollectionContext) -> list[Dependency]:
    dep_ctx = DepCollectionContext(
        advanced_unstable_linking = dep_ctx.advanced_unstable_linking,
        # Include doc deps here for any doctests that may be present in the target.
        include_doc_deps = True,
        is_proc_macro = dep_ctx.is_proc_macro,
        # Rust Analyzer handles the sysroot separately. We omit the sysroot deps here and will
        # instead pass a path to the sysroot as a separate config.
        explicit_sysroot_deps = None,
        panic_runtime = dep_ctx.panic_runtime,
    )

    first_order_deps = resolve_rust_deps(ctx, dep_ctx)
    available_proc_macros = get_available_proc_macros(ctx)

    return [dep.dep for dep in first_order_deps] + available_proc_macros.values()

def rust_analyzer_provider(
        ctx: AnalysisContext,
        dep_ctx: DepCollectionContext,
        default_roots: list[str]) -> RustAnalyzerInfo:
    return RustAnalyzerInfo(
        crate_root = crate_root(ctx, default_roots),
        rust_deps = _compute_rust_deps(ctx, dep_ctx),
    )
