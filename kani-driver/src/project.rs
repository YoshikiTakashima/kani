// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module defines the structure for a Kani project.
//! The goal is to provide one project view independent on the build system (cargo / standalone
//! rustc) and its configuration (e.g.: linker type).
//!
//! For `--function`, we still have a hack in-place that merges all the artifacts together.
//! The reason is the following:
//!  - For `--function`, the compiler doesn't generate any metadata that indicates which
//!    functions each goto model includes. Thus, we don't have an easy way to tell which goto
//!    files are relevant for the function verification. This is also another flag that we don't
//!    expect to stabilize, so we also opted to use the same hack as implemented before the MIR
//!    Linker was introduced to merge everything together.
//!
//! Note that for `--function` we also inject a mock `HarnessMetadata` to the project. This
//! allows the rest of the driver to handle a function under verification the same way it handle
//! other harnesses.

use crate::metadata::{from_json, merge_kani_metadata, mock_proof_harness};
use crate::session::KaniSession;
use crate::util::{crate_name, guess_rlib_name};
use anyhow::{Context, Result};
use kani_metadata::{
    artifact::convert_type, ArtifactType, ArtifactType::*, HarnessMetadata, KaniMetadata,
};
use std::fs::File;
use std::io::BufWriter;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

/// This structure represent the project information relevant for verification.
/// A `Project` contains information about all crates under verification, as well as all
/// artifacts relevant for verification.
///
/// For one specific harness, there should be up to one artifact of each type. I.e., artifacts of
/// the same type are linked as part of creating the project.
///
/// However, one artifact can be used for multiple harnesses. This will depend on the type of
/// artifact, but it should be transparent for the user of this object.
#[derive(Debug, Default)]
pub struct Project {
    /// Each target crate metadata.
    pub metadata: Vec<KaniMetadata>,
    /// The directory where all outputs should be directed to. This path represents the canonical
    /// version of outdir.
    pub outdir: PathBuf,
    /// The collection of artifacts kept as part of this project.
    artifacts: Vec<Artifact>,
    /// A flag that indicated whether all artifacts have been merged or not.
    ///
    /// This allow us to provide a consistent behavior for `--function`.
    /// For this option, we still merge all the artifacts together, so the
    /// `merged_artifacts` flag will be set to `true`.
    /// When this flag is `true`, there should only be up to one artifact of any given type.
    /// When this flag is `false`, there may be multiple artifacts for any given type. However,
    /// only up to one artifact for each
    pub merged_artifacts: bool,
    /// Records the cargo metadata from the build, if there was any
    pub cargo_metadata: Option<cargo_metadata::Metadata>,
    /// For build `keep_going` mode, we collect the targets that we failed to compile.
    pub failed_targets: Option<Vec<String>>,
}

impl Project {
    /// Get all harnesses from a project. This will include all test and proof harnesses.
    /// We could create a `get_proof_harnesses` and a `get_tests_harnesses` later if we see the
    /// need to split them.
    pub fn get_all_harnesses(&self) -> Vec<&HarnessMetadata> {
        self.metadata
            .iter()
            .flat_map(|crate_metadata| {
                crate_metadata.proof_harnesses.iter().chain(crate_metadata.test_harnesses.iter())
            })
            .collect()
    }

    /// Return the matching artifact for the given harness.
    ///
    /// If the harness has information about the goto_file we can use that to find the exact file.
    /// For cases where there is no goto_file, we just assume that everything has been linked
    /// together. I.e.: There should only be one artifact of the given type.
    pub fn get_harness_artifact(
        &self,
        harness: &HarnessMetadata,
        typ: ArtifactType,
    ) -> Option<&Artifact> {
        let expected_path = if self.merged_artifacts {
            None
        } else {
            harness
                .goto_file
                .as_ref()
                .and_then(|goto_file| convert_type(goto_file, SymTabGoto, typ).canonicalize().ok())
        };
        trace!(?harness.goto_file, ?expected_path, ?typ, "get_harness_artifact");
        self.artifacts.iter().find(|artifact| {
            artifact.has_type(typ)
                && expected_path.as_ref().map_or(true, |goto_file| *goto_file == artifact.path)
        })
    }

    /// Try to build a new project from the build result metadata.
    ///
    /// This method will parse the metadata in order to gather all artifacts generated by the
    /// compiler.
    fn try_new(
        session: &KaniSession,
        outdir: PathBuf,
        metadata: Vec<KaniMetadata>,
        cargo_metadata: Option<cargo_metadata::Metadata>,
        failed_targets: Option<Vec<String>>,
    ) -> Result<Self> {
        // For each harness (test or proof) from each metadata, read the path for the goto
        // SymTabGoto file. Use that path to find all the other artifacts.
        let mut artifacts = vec![];
        for crate_metadata in &metadata {
            for harness_metadata in
                crate_metadata.test_harnesses.iter().chain(crate_metadata.proof_harnesses.iter())
            {
                let symtab_out = Artifact::try_new(
                    harness_metadata.goto_file.as_ref().expect("Expected a model file"),
                    SymTabGoto,
                )?;
                let goto_path = convert_type(&symtab_out.path, symtab_out.typ, Goto);

                // Link
                session.link_goto_binary(&[symtab_out.to_path_buf()], &goto_path)?;
                let goto = Artifact::try_new(&goto_path, Goto)?;

                // All other harness artifacts that may have been generated as part of the build.
                artifacts.extend(
                    [SymTab, TypeMap, VTableRestriction, PrettyNameMap].iter().filter_map(|typ| {
                        let artifact = Artifact::try_from(&symtab_out, *typ).ok()?;
                        Some(artifact)
                    }),
                );
                artifacts.push(symtab_out);
                artifacts.push(goto);
            }
        }

        Ok(Project {
            outdir,
            metadata,
            artifacts,
            merged_artifacts: false,
            cargo_metadata,
            failed_targets,
        })
    }
}

/// Information about a build artifact.
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Artifact {
    /// The path for this artifact in the canonical form.
    path: PathBuf,
    /// The type of artifact.
    typ: ArtifactType,
}

impl AsRef<Path> for Artifact {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for Artifact {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Artifact {
    /// Create a new artifact if the given path exists.
    pub fn try_new(path: &Path, typ: ArtifactType) -> Result<Self> {
        Ok(Artifact {
            path: path.canonicalize().context(format!("Failed to process {}", path.display()))?,
            typ,
        })
    }

    /// Check if this artifact has the given type.
    pub fn has_type(&self, typ: ArtifactType) -> bool {
        self.typ == typ
    }

    /// Try to derive an artifact based on a different artifact of a different type.
    /// For example:
    /// ```no_run
    /// let artifact = Artifact::try_new(&"/tmp/file.kani_metadata.json", Metadata).unwrap();
    /// let goto = Artifact::try_from(artifact, Goto); // Will try to create "/tmp/file.goto"
    /// ```
    pub fn try_from(artifact: &Artifact, typ: ArtifactType) -> Result<Self> {
        Self::try_new(&convert_type(&artifact.path, artifact.typ, typ), typ)
    }
}

/// Store the KaniMetadata into a file.
fn dump_metadata(metadata: &KaniMetadata, path: &Path) {
    let out_file = File::create(path).unwrap();
    let writer = BufWriter::new(out_file);
    serde_json::to_writer_pretty(writer, &metadata).unwrap();
}

/// Generate a project using `cargo`.
/// Accept a boolean to build as many targets as possible. The number of failures in that case can
/// be collected from the project.
pub fn cargo_project(session: &KaniSession, keep_going: bool) -> Result<Project> {
    let outputs = session.cargo_build(keep_going)?;
    let outdir = outputs.outdir.canonicalize()?;
    if session.args.function.is_some() {
        let mut artifacts = vec![];
        // For the `--function` support, we still use a glob to link everything.
        // Yes, this is broken, but it has been broken for quite some time. :(
        // Merge goto files.
        // https://github.com/model-checking/kani/issues/2129
        let joined_name = "cbmc-linked";
        let base_name = outdir.join(joined_name);
        let goto = base_name.with_extension(Goto);
        let all_gotos = outputs
            .metadata
            .iter()
            .map(|artifact| convert_type(&artifact, Metadata, SymTabGoto))
            .collect::<Vec<_>>();
        session.link_goto_binary(&all_gotos, &goto)?;
        let goto_artifact = Artifact::try_new(&goto, Goto)?;

        // Merge metadata files.
        let per_crate: Vec<_> =
            outputs.metadata.iter().filter_map(|f| from_json::<KaniMetadata>(f).ok()).collect();
        let merged_metadata = merge_kani_metadata(per_crate);
        let metadata = metadata_with_function(
            session,
            joined_name,
            merged_metadata,
            goto_artifact.with_extension(SymTabGoto),
        );
        let metadata_file = base_name.with_extension(Metadata);
        dump_metadata(&metadata, &metadata_file);
        artifacts.push(goto_artifact);
        artifacts.push(Artifact::try_new(&metadata_file, Metadata)?);

        Ok(Project {
            outdir,
            artifacts,
            metadata: vec![metadata],
            merged_artifacts: true,
            cargo_metadata: Some(outputs.cargo_metadata),
            failed_targets: outputs.failed_targets,
        })
    } else {
        // For the MIR Linker we know there is only one metadata per crate. Use that in our favor.
        let metadata = outputs
            .metadata
            .iter()
            .map(|md_file| from_json(md_file))
            .collect::<Result<Vec<_>>>()?;
        Project::try_new(
            session,
            outdir,
            metadata,
            Some(outputs.cargo_metadata),
            outputs.failed_targets,
        )
    }
}

/// Generate a project directly using `kani-compiler` on a single crate.
pub fn standalone_project(input: &Path, session: &KaniSession) -> Result<Project> {
    StandaloneProjectBuilder::try_new(input, session)?.build()
}

/// Builder for a standalone project.
struct StandaloneProjectBuilder<'a> {
    /// The directory where all outputs should be directed to.
    outdir: PathBuf,
    /// The metadata file for the target crate.
    metadata: Artifact,
    /// The input file.
    input: PathBuf,
    /// The crate name.
    crate_name: String,
    /// The Kani session.
    session: &'a KaniSession,
}

impl<'a> StandaloneProjectBuilder<'a> {
    /// Create a `StandaloneProjectBuilder` from the given input and session.
    /// This will perform a few validations before the build.
    fn try_new(input: &Path, session: &'a KaniSession) -> Result<Self> {
        // Ensure the directory exist and it's in its canonical form.
        let outdir = if let Some(target_dir) = &session.args.target_dir {
            std::fs::create_dir_all(target_dir)?; // This is a no-op if directory exists.
            target_dir.canonicalize()?
        } else {
            input.canonicalize().unwrap().parent().unwrap().to_path_buf()
        };
        let crate_name = crate_name(&input);
        let metadata = standalone_artifact(&outdir, &crate_name, Metadata);
        Ok(StandaloneProjectBuilder {
            outdir,
            metadata,
            input: input.to_path_buf(),
            crate_name,
            session,
        })
    }

    /// Build a project by compiling `self.input` file.
    fn build(self) -> Result<Project> {
        // Register artifacts that may be generated by the compiler / linker for future deletion.
        let rlib_path = guess_rlib_name(&self.outdir.join(self.input.file_name().unwrap()));
        self.session.record_temporary_file(&rlib_path);
        self.session.record_temporary_file(&self.metadata.path);

        // Build and link the artifacts.
        debug!(krate=?self.crate_name, input=?self.input, ?rlib_path, "build compile");
        self.session.compile_single_rust_file(&self.input, &self.crate_name, &self.outdir)?;

        let metadata = if let Ok(goto_model) = Artifact::try_from(&self.metadata, SymTabGoto) {
            metadata_with_function(
                self.session,
                &self.crate_name,
                from_json(&self.metadata)?,
                goto_model.path,
            )
        } else {
            from_json(&self.metadata)?
        };

        // Create the project with the artifacts built by the compiler.
        let result = Project::try_new(self.session, self.outdir, vec![metadata], None, None);
        if let Ok(project) = &result {
            self.session.record_temporary_files(&project.artifacts);
        }
        result
    }
}

/// Generate a `KaniMetadata` by extending the original metadata to contain the function under
/// verification, when there is one.
fn metadata_with_function(
    session: &KaniSession,
    crate_name: &str,
    mut metadata: KaniMetadata,
    model_file: PathBuf,
) -> KaniMetadata {
    if let Some(name) = &session.args.function {
        // --function is untranslated, create a mock harness
        metadata.proof_harnesses.push(mock_proof_harness(
            name,
            None,
            Some(crate_name),
            Some(model_file),
        ));
    }
    metadata
}

/// Generate the expected path of a standalone artifact of the given type.
// Note: `out_dir` is already on canonical form, so no need to invoke `try_new()`.
fn standalone_artifact(out_dir: &Path, crate_name: &String, typ: ArtifactType) -> Artifact {
    let mut path = out_dir.join(crate_name);
    let _ = path.set_extension(&typ);
    Artifact { path, typ }
}
