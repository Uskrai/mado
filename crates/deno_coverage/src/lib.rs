// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
// forked from https://github.com/denoland/deno/blob/5f5bbd597ad3454469b3e51a80cd7cb9be39c04d/cli/tools/coverage/mod.rs

use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::LocalInspectorSession;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

mod json_types;

use json_types::*;

pub struct CoverageCollector {
    pub dir: PathBuf,
    session: LocalInspectorSession,
}

impl CoverageCollector {
    pub fn new(dir: PathBuf, session: LocalInspectorSession) -> Self {
        Self { dir, session }
    }

    async fn enable_debugger(&mut self) -> Result<(), AnyError> {
        self.session
            .post_message::<()>("Debugger.enable", None)
            .await?;
        Ok(())
    }

    async fn enable_profiler(&mut self) -> Result<(), AnyError> {
        self.session
            .post_message::<()>("Profiler.enable", None)
            .await?;
        Ok(())
    }

    async fn disable_debugger(&mut self) -> Result<(), AnyError> {
        self.session
            .post_message::<()>("Debugger.disable", None)
            .await?;
        Ok(())
    }

    async fn disable_profiler(&mut self) -> Result<(), AnyError> {
        self.session
            .post_message::<()>("Profiler.disable", None)
            .await?;
        Ok(())
    }

    async fn start_precise_coverage(
        &mut self,
        parameters: StartPreciseCoverageParameters,
    ) -> Result<StartPreciseCoverageReturnObject, AnyError> {
        let return_value = self
            .session
            .post_message("Profiler.startPreciseCoverage", Some(parameters))
            .await?;

        let return_object = serde_json::from_value(return_value)?;

        Ok(return_object)
    }

    async fn take_precise_coverage(&mut self) -> Result<TakePreciseCoverageReturnObject, AnyError> {
        let return_value = self
            .session
            .post_message::<()>("Profiler.takePreciseCoverage", None)
            .await?;

        let return_object = serde_json::from_value(return_value)?;

        Ok(return_object)
    }

    pub async fn start_collecting(&mut self) -> Result<(), AnyError> {
        self.enable_debugger().await?;
        self.enable_profiler().await?;
        self.start_precise_coverage(StartPreciseCoverageParameters {
            call_count: true,
            detailed: true,
            allow_triggered_updates: false,
        })
        .await?;

        Ok(())
    }

    pub async fn stop_collecting(&mut self) -> Result<(), AnyError> {
        fs::create_dir_all(&self.dir)?;

        let script_coverages = self.take_precise_coverage().await?.result;
        for script_coverage in script_coverages {
            let filename = format!("{}.json", Uuid::new_v4());
            let filepath = self.dir.join(filename);

            let mut out = BufWriter::new(File::create(filepath)?);
            let formated_coverage = serde_json::to_string_pretty(&script_coverage)?;

            out.write_all(formated_coverage.as_bytes())?;
            out.flush()?;
        }

        self.disable_debugger().await?;
        self.disable_profiler().await?;

        Ok(())
    }
}
