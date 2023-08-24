//! Settings for the `isort` plugin.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::hash::BuildHasherDefault;

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use ruff_macros::{CacheKey, CombineOptions, ConfigurationOptions};

use crate::rules::isort::categorize::KnownModules;
use crate::rules::isort::ImportType;
use crate::settings::types::IdentifierPattern;
use crate::warn_user_once;

use super::categorize::ImportSection;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, CacheKey)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum RelativeImportsOrder {
    /// Place "closer" imports (fewer `.` characters, most local) before
    /// "further" imports (more `.` characters, least local).
    ClosestToFurthest,
    /// Place "further" imports (more `.` characters, least local) imports
    /// before "closer" imports (fewer `.` characters, most local).
    FurthestToClosest,
}

impl Default for RelativeImportsOrder {
    fn default() -> Self {
        Self::FurthestToClosest
    }
}

#[derive(
    Debug, PartialEq, Eq, Serialize, Deserialize, Default, ConfigurationOptions, CombineOptions,
)]
#[serde(
    deny_unknown_fields,
    rename_all = "kebab-case",
    rename = "IsortOptions"
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Options {
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"
            force-wrap-aliases = true
            combine-as-imports = true
        "#
    )]
    /// Force `import from` statements with multiple members and at least one
    /// alias (e.g., `import A as B`) to wrap such that every line contains
    /// exactly one member. For example, this formatting would be retained,
    /// rather than condensing to a single line:
    ///
    /// ```python
    /// from .utils import (
    ///     test_directory as test_directory,
    ///     test_id as test_id
    /// )
    /// ```
    ///
    /// Note that this setting is only effective when combined with
    /// `combine-as-imports = true`. When `combine-as-imports` isn't
    /// enabled, every aliased `import from` will be given its own line, in
    /// which case, wrapping is not necessary.
    pub force_wrap_aliases: Option<bool>,
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"force-single-line = true"#
    )]
    /// Forces all from imports to appear on their own line.
    pub force_single_line: Option<bool>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            single-line-exclusions = ["os", "json"]
        "#
    )]
    /// One or more modules to exclude from the single line rule.
    pub single_line_exclusions: Option<Vec<String>>,
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"
            combine-as-imports = true
        "#
    )]
    /// Combines as imports on the same line. See isort's [`combine-as-imports`](https://pycqa.github.io/isort/docs/configuration/options.html#combine-as-imports)
    /// option.
    pub combine_as_imports: Option<bool>,
    #[option(
        default = r#"true"#,
        value_type = "bool",
        example = r#"
            split-on-trailing-comma = false
        "#
    )]
    /// If a comma is placed after the last member in a multi-line import, then
    /// the imports will never be folded into one line.
    ///
    /// See isort's [`split-on-trailing-comma`](https://pycqa.github.io/isort/docs/configuration/options.html#split-on-trailing-comma) option.
    pub split_on_trailing_comma: Option<bool>,
    #[option(
        default = r#"true"#,
        value_type = "bool",
        example = r#"
            order-by-type = true
        "#
    )]
    /// Order imports by type, which is determined by case, in addition to
    /// alphabetically.
    pub order_by_type: Option<bool>,
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"
            force-sort-within-sections = true
        "#
    )]
    /// Don't sort straight-style imports (like `import sys`) before from-style
    /// imports (like `from itertools import groupby`). Instead, sort the
    /// imports by module, independent of import style.
    pub force_sort_within_sections: Option<bool>,
    #[option(
        default = r#"false"#,
        value_type = "bool",
        example = r#"
            case-sensitive = true
        "#
    )]
    /// Sort imports taking into account case sensitivity.
    pub case_sensitive: Option<bool>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            force-to-top = ["src"]
        "#
    )]
    /// Force specific imports to the top of their appropriate section.
    pub force_to_top: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            known-first-party = ["src"]
        "#
    )]
    /// A list of modules to consider first-party, regardless of whether they
    /// can be identified as such via introspection of the local filesystem.
    ///
    /// Supports glob patterns. For more information on the glob syntax, refer
    /// to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    pub known_first_party: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            known-third-party = ["src"]
        "#
    )]
    /// A list of modules to consider third-party, regardless of whether they
    /// can be identified as such via introspection of the local filesystem.
    ///
    /// Supports glob patterns. For more information on the glob syntax, refer
    /// to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    pub known_third_party: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            known-local-folder = ["src"]
        "#
    )]
    /// A list of modules to consider being a local folder.
    /// Generally, this is reserved for relative imports (`from . import module`).
    ///
    /// Supports glob patterns. For more information on the glob syntax, refer
    /// to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    pub known_local_folder: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            extra-standard-library = ["path"]
        "#
    )]
    /// A list of modules to consider standard-library, in addition to those
    /// known to Ruff in advance.
    ///
    /// Supports glob patterns. For more information on the glob syntax, refer
    /// to the [`globset` documentation](https://docs.rs/globset/latest/globset/#syntax).
    pub extra_standard_library: Option<Vec<String>>,
    #[option(
        default = r#"furthest-to-closest"#,
        value_type = r#""furthest-to-closest" | "closest-to-furthest""#,
        example = r#"
            relative-imports-order = "closest-to-furthest"
        "#
    )]
    /// Whether to place "closer" imports (fewer `.` characters, most local)
    /// before "further" imports (more `.` characters, least local), or vice
    /// versa.
    ///
    /// The default ("furthest-to-closest") is equivalent to isort's
    /// `reverse-relative` default (`reverse-relative = false`); setting
    /// this to "closest-to-furthest" is equivalent to isort's
    /// `reverse-relative = true`.
    pub relative_imports_order: Option<RelativeImportsOrder>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            required-imports = ["from __future__ import annotations"]
        "#
    )]
    /// Add the specified import line to all files.
    pub required_imports: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            classes = ["SVC"]
        "#
    )]
    /// An override list of tokens to always recognize as a Class for
    /// `order-by-type` regardless of casing.
    pub classes: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            constants = ["constant"]
        "#
    )]
    /// An override list of tokens to always recognize as a CONSTANT
    /// for `order-by-type` regardless of casing.
    pub constants: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            variables = ["VAR"]
        "#
    )]
    /// An override list of tokens to always recognize as a var
    /// for `order-by-type` regardless of casing.
    pub variables: Option<Vec<String>>,
    #[option(
        default = r#"[]"#,
        value_type = r#"list["future" | "standard-library" | "third-party" | "first-party" | "local-folder" | str]"#,
        example = r#"
            no-lines-before = ["future", "standard-library"]
        "#
    )]
    /// A list of sections that should _not_ be delineated from the previous
    /// section via empty lines.
    pub no_lines_before: Option<Vec<ImportSection>>,
    #[option(
        default = r#"-1"#,
        value_type = "int",
        example = r#"
            # Use a single line after each import block.
            lines-after-imports = 1
        "#
    )]
    /// The number of blank lines to place after imports.
    /// Use `-1` for automatic determination.
    pub lines_after_imports: Option<isize>,
    #[option(
        default = r#"0"#,
        value_type = "int",
        example = r#"
            # Use a single line between direct and from import
            lines-between-types = 1
        "#
    )]
    /// The number of lines to place between "direct" and `import from` imports.
    pub lines_between_types: Option<usize>,
    #[option(
        default = r#"[]"#,
        value_type = "list[str]",
        example = r#"
            forced-separate = ["tests"]
        "#
    )]
    /// A list of modules to separate into auxiliary block(s) of imports,
    /// in the order specified.
    pub forced_separate: Option<Vec<String>>,
    #[option(
        default = r#"["future", "standard-library", "third-party", "first-party", "local-folder"]"#,
        value_type = r#"list["future" | "standard-library" | "third-party" | "first-party" | "local-folder" | str]"#,
        example = r#"
            section-order = ["future", "standard-library", "first-party", "local-folder", "third-party"]
        "#
    )]
    /// Override in which order the sections should be output. Can be used to move custom sections.
    pub section_order: Option<Vec<ImportSection>>,
    #[option(
        default = r#"true"#,
        value_type = "bool",
        example = r#"
            detect-same-package = false
        "#
    )]
    /// Whether to automatically mark imports from within the same package as first-party.
    /// For example, when `detect-same-package = true`, then when analyzing files within the
    /// `foo` package, any imports from within the `foo` package will be considered first-party.
    ///
    /// This heuristic is often unnecessary when `src` is configured to detect all first-party
    /// sources; however, if `src` is _not_ configured, this heuristic can be useful to detect
    /// first-party imports from _within_ (but not _across_) first-party packages.
    pub detect_same_package: Option<bool>,
    // Tables are required to go last.
    #[option(
        default = "{}",
        value_type = "dict[str, list[str]]",
        example = r#"
            # Group all Django imports into a separate section.
            [tool.ruff.isort.sections]
            "django" = ["django"]
        "#
    )]
    /// A list of mappings from section names to modules.
    /// By default custom sections are output last, but this can be overridden with `section-order`.
    pub sections: Option<FxHashMap<ImportSection, Vec<String>>>,
}

#[derive(Debug, CacheKey)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub required_imports: BTreeSet<String>,
    pub combine_as_imports: bool,
    pub force_single_line: bool,
    pub force_sort_within_sections: bool,
    pub case_sensitive: bool,
    pub force_wrap_aliases: bool,
    pub force_to_top: BTreeSet<String>,
    pub known_modules: KnownModules,
    pub detect_same_package: bool,
    pub order_by_type: bool,
    pub relative_imports_order: RelativeImportsOrder,
    pub single_line_exclusions: BTreeSet<String>,
    pub split_on_trailing_comma: bool,
    pub classes: BTreeSet<String>,
    pub constants: BTreeSet<String>,
    pub variables: BTreeSet<String>,
    pub no_lines_before: BTreeSet<ImportSection>,
    pub lines_after_imports: isize,
    pub lines_between_types: usize,
    pub forced_separate: Vec<String>,
    pub section_order: Vec<ImportSection>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            required_imports: BTreeSet::new(),
            combine_as_imports: false,
            force_single_line: false,
            force_sort_within_sections: false,
            detect_same_package: true,
            case_sensitive: false,
            force_wrap_aliases: false,
            force_to_top: BTreeSet::new(),
            known_modules: KnownModules::default(),
            order_by_type: true,
            relative_imports_order: RelativeImportsOrder::default(),
            single_line_exclusions: BTreeSet::new(),
            split_on_trailing_comma: true,
            classes: BTreeSet::new(),
            constants: BTreeSet::new(),
            variables: BTreeSet::new(),
            no_lines_before: BTreeSet::new(),
            lines_after_imports: -1,
            lines_between_types: 0,
            forced_separate: Vec::new(),
            section_order: ImportType::iter().map(ImportSection::Known).collect(),
        }
    }
}

impl TryFrom<Options> for Settings {
    type Error = SettingsError;

    fn try_from(options: Options) -> Result<Self, Self::Error> {
        // Extract any configuration options that deal with user-defined sections.
        let mut section_order: Vec<_> = options
            .section_order
            .unwrap_or_else(|| ImportType::iter().map(ImportSection::Known).collect());
        let known_first_party = options
            .known_first_party
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| IdentifierPattern::new(&name))
                    .collect()
            })
            .transpose()
            .map_err(SettingsError::InvalidKnownFirstParty)?
            .unwrap_or_default();
        let known_third_party = options
            .known_third_party
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| IdentifierPattern::new(&name))
                    .collect()
            })
            .transpose()
            .map_err(SettingsError::InvalidKnownThirdParty)?
            .unwrap_or_default();
        let known_local_folder = options
            .known_local_folder
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| IdentifierPattern::new(&name))
                    .collect()
            })
            .transpose()
            .map_err(SettingsError::InvalidKnownLocalFolder)?
            .unwrap_or_default();
        let extra_standard_library = options
            .extra_standard_library
            .map(|names| {
                names
                    .into_iter()
                    .map(|name| IdentifierPattern::new(&name))
                    .collect()
            })
            .transpose()
            .map_err(SettingsError::InvalidExtraStandardLibrary)?
            .unwrap_or_default();
        let no_lines_before = options.no_lines_before.unwrap_or_default();
        let sections = options.sections.unwrap_or_default();

        // Verify that `sections` doesn't contain any built-in sections.
        let sections: FxHashMap<String, Vec<glob::Pattern>> = sections
            .into_iter()
            .filter_map(|(section, modules)| match section {
                ImportSection::Known(section) => {
                    warn_user_once!("`sections` contains built-in section: `{:?}`", section);
                    None
                }
                ImportSection::UserDefined(section) => Some((section, modules)),
            })
            .map(|(section, modules)| {
                let modules = modules
                    .into_iter()
                    .map(|module| {
                        IdentifierPattern::new(&module)
                            .map_err(SettingsError::InvalidUserDefinedSection)
                    })
                    .collect::<Result<Vec<_>, Self::Error>>()?;
                Ok((section, modules))
            })
            .collect::<Result<_, _>>()?;

        // Verify that `section_order` doesn't contain any duplicates.
        let mut seen =
            FxHashSet::with_capacity_and_hasher(section_order.len(), BuildHasherDefault::default());
        for section in &section_order {
            if !seen.insert(section) {
                warn_user_once!(
                    "`section-order` contains duplicate section: `{:?}`",
                    section
                );
            }
        }

        // Verify that all sections listed in `section_order` are defined in `sections`.
        for section in &section_order {
            if let ImportSection::UserDefined(section_name) = section {
                if !sections.contains_key(section_name) {
                    warn_user_once!("`section-order` contains unknown section: `{:?}`", section,);
                }
            }
        }

        // Verify that all sections listed in `no_lines_before` are defined in `sections`.
        for section in &no_lines_before {
            if let ImportSection::UserDefined(section_name) = section {
                if !sections.contains_key(section_name) {
                    warn_user_once!(
                        "`no-lines-before` contains unknown section: `{:?}`",
                        section,
                    );
                }
            }
        }

        // Add all built-in sections to `section_order`, if not already present.
        for section in ImportType::iter().map(ImportSection::Known) {
            if !section_order.contains(&section) {
                warn_user_once!(
                    "`section-order` is missing built-in section: `{:?}`",
                    section
                );
                section_order.push(section);
            }
        }

        // Add all user-defined sections to `section-order`, if not already present.
        for section_name in sections.keys() {
            let section = ImportSection::UserDefined(section_name.clone());
            if !section_order.contains(&section) {
                warn_user_once!("`section-order` is missing section: `{:?}`", section);
                section_order.push(section);
            }
        }

        Ok(Self {
            required_imports: BTreeSet::from_iter(options.required_imports.unwrap_or_default()),
            combine_as_imports: options.combine_as_imports.unwrap_or(false),
            force_single_line: options.force_single_line.unwrap_or(false),
            force_sort_within_sections: options.force_sort_within_sections.unwrap_or(false),
            case_sensitive: options.case_sensitive.unwrap_or(false),
            force_wrap_aliases: options.force_wrap_aliases.unwrap_or(false),
            detect_same_package: options.detect_same_package.unwrap_or(true),
            force_to_top: BTreeSet::from_iter(options.force_to_top.unwrap_or_default()),
            known_modules: KnownModules::new(
                known_first_party,
                known_third_party,
                known_local_folder,
                extra_standard_library,
                sections,
            ),
            order_by_type: options.order_by_type.unwrap_or(true),
            relative_imports_order: options.relative_imports_order.unwrap_or_default(),
            single_line_exclusions: BTreeSet::from_iter(
                options.single_line_exclusions.unwrap_or_default(),
            ),
            split_on_trailing_comma: options.split_on_trailing_comma.unwrap_or(true),
            classes: BTreeSet::from_iter(options.classes.unwrap_or_default()),
            constants: BTreeSet::from_iter(options.constants.unwrap_or_default()),
            variables: BTreeSet::from_iter(options.variables.unwrap_or_default()),
            no_lines_before: BTreeSet::from_iter(no_lines_before),
            lines_after_imports: options.lines_after_imports.unwrap_or(-1),
            lines_between_types: options.lines_between_types.unwrap_or_default(),
            forced_separate: Vec::from_iter(options.forced_separate.unwrap_or_default()),
            section_order,
        })
    }
}

/// Error returned by the [`TryFrom`] implementation of [`Settings`].
#[derive(Debug)]
pub enum SettingsError {
    InvalidKnownFirstParty(glob::PatternError),
    InvalidKnownThirdParty(glob::PatternError),
    InvalidKnownLocalFolder(glob::PatternError),
    InvalidExtraStandardLibrary(glob::PatternError),
    InvalidUserDefinedSection(glob::PatternError),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::InvalidKnownThirdParty(err) => {
                write!(f, "invalid known third-party pattern: {err}")
            }
            SettingsError::InvalidKnownFirstParty(err) => {
                write!(f, "invalid known first-party pattern: {err}")
            }
            SettingsError::InvalidKnownLocalFolder(err) => {
                write!(f, "invalid known local folder pattern: {err}")
            }
            SettingsError::InvalidExtraStandardLibrary(err) => {
                write!(f, "invalid extra standard library pattern: {err}")
            }
            SettingsError::InvalidUserDefinedSection(err) => {
                write!(f, "invalid user-defined section pattern: {err}")
            }
        }
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SettingsError::InvalidKnownThirdParty(err) => Some(err),
            SettingsError::InvalidKnownFirstParty(err) => Some(err),
            SettingsError::InvalidKnownLocalFolder(err) => Some(err),
            SettingsError::InvalidExtraStandardLibrary(err) => Some(err),
            SettingsError::InvalidUserDefinedSection(err) => Some(err),
        }
    }
}

impl From<Settings> for Options {
    fn from(settings: Settings) -> Self {
        Self {
            required_imports: Some(settings.required_imports.into_iter().collect()),
            combine_as_imports: Some(settings.combine_as_imports),
            extra_standard_library: Some(
                settings
                    .known_modules
                    .modules_for_known_type(ImportType::StandardLibrary)
                    .map(ToString::to_string)
                    .collect(),
            ),
            force_single_line: Some(settings.force_single_line),
            force_sort_within_sections: Some(settings.force_sort_within_sections),
            case_sensitive: Some(settings.case_sensitive),
            force_wrap_aliases: Some(settings.force_wrap_aliases),
            detect_same_package: Some(settings.detect_same_package),
            force_to_top: Some(settings.force_to_top.into_iter().collect()),
            known_first_party: Some(
                settings
                    .known_modules
                    .modules_for_known_type(ImportType::FirstParty)
                    .map(ToString::to_string)
                    .collect(),
            ),
            known_third_party: Some(
                settings
                    .known_modules
                    .modules_for_known_type(ImportType::ThirdParty)
                    .map(ToString::to_string)
                    .collect(),
            ),
            known_local_folder: Some(
                settings
                    .known_modules
                    .modules_for_known_type(ImportType::LocalFolder)
                    .map(ToString::to_string)
                    .collect(),
            ),
            order_by_type: Some(settings.order_by_type),
            relative_imports_order: Some(settings.relative_imports_order),
            single_line_exclusions: Some(settings.single_line_exclusions.into_iter().collect()),
            split_on_trailing_comma: Some(settings.split_on_trailing_comma),
            classes: Some(settings.classes.into_iter().collect()),
            constants: Some(settings.constants.into_iter().collect()),
            variables: Some(settings.variables.into_iter().collect()),
            no_lines_before: Some(settings.no_lines_before.into_iter().collect()),
            lines_after_imports: Some(settings.lines_after_imports),
            lines_between_types: Some(settings.lines_between_types),
            forced_separate: Some(settings.forced_separate.into_iter().collect()),
            section_order: Some(settings.section_order.into_iter().collect()),
            sections: Some(
                settings
                    .known_modules
                    .user_defined()
                    .into_iter()
                    .map(|(section, modules)| {
                        (
                            ImportSection::UserDefined(section.to_string()),
                            modules.into_iter().map(ToString::to_string).collect(),
                        )
                    })
                    .collect(),
            ),
        }
    }
}
