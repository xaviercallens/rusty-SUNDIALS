//! SUNDIALS context management (Rust translation of `sundials_context.c`).
//!
//! This module provides an idiomatic, safe Rust implementation of the SUNContext
//! lifecycle and associated services (logger, profiler, error handlers), preserving
//! the C control flow and semantics where applicable.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

/// SUNDIALS communication handle (opaque in C).
pub type SunComm = i32;

/// Result alias for SUNContext operations.
pub type SunResult<T> = Result<T, SunErrCode>;

/// Error codes corresponding to SUNDIALS `SUNErrCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunErrCode {
    Success,
    MallocFail,
    SunctxCorrupt,
    Corrupt,
    DestroyFail,
    LoggerFail,
    ProfilerFail,
    ErrHandlerFail,
}

/// Logger abstraction.
pub trait SunLogger: Send + Sync {
    #[inline]
    fn set_error_filename(&self, _name: &str) -> SunResult<()> {
        Ok(())
    }
    #[inline]
    fn set_warning_filename(&self, _name: &str) -> SunResult<()> {
        Ok(())
    }
    #[inline]
    fn set_info_filename(&self, _name: &str) -> SunResult<()> {
        Ok(())
    }
    #[inline]
    fn set_debug_filename(&self, _name: &str) -> SunResult<()> {
        Ok(())
    }
}

/// Profiler abstraction.
pub trait SunProfiler: Send + Sync {
    fn print(&self, w: &mut dyn Write) -> SunResult<()>;
}

/// Error handler callback.
pub type ErrHandlerFn = Arc<dyn Fn(SunErrCode, &str) + Send + Sync>;

struct ErrHandlerNode {
    f: ErrHandlerFn,
    user_data: Option<Arc<dyn Send + Sync>>,
    previous: Option<Box<ErrHandlerNode>>,
}

/// Runtime feature/config flags replacing C preprocessor branches.
#[derive(Debug, Clone)]
pub struct SunContextConfig {
    pub logging_level: usize,
    pub mpi_enabled: bool,
    pub profiling_enabled: bool,
    pub caliper_enabled: bool,
}

impl Default for SunContextConfig {
    #[inline]
    fn default() -> Self {
        Self {
            logging_level: 0,
            mpi_enabled: false,
            profiling_enabled: false,
            caliper_enabled: false,
        }
    }
}

/// Builder for `SunContext`.
pub struct SunContextBuilder {
    comm: SunComm,
    config: SunContextConfig,
    logger_factory: Option<Arc<dyn Fn(SunComm) -> SunResult<Arc<dyn SunLogger>> + Send + Sync>>,
    profiler_factory:
        Option<Arc<dyn Fn(SunComm, &str) -> SunResult<Arc<dyn SunProfiler>> + Send + Sync>>,
    default_err_handler: Option<ErrHandlerFn>,
}

impl SunContextBuilder {
    #[inline]
    pub const fn new(comm: SunComm) -> Self {
        Self {
            comm,
            config: SunContextConfig {
                logging_level: 0,
                mpi_enabled: false,
                profiling_enabled: false,
                caliper_enabled: false,
            },
            logger_factory: None,
            profiler_factory: None,
            default_err_handler: None,
        }
    }

    #[inline]
    pub fn config(mut self, cfg: SunContextConfig) -> Self {
        self.config = cfg;
        self
    }

    #[inline]
    pub fn logger_factory(
        mut self,
        f: Arc<dyn Fn(SunComm) -> SunResult<Arc<dyn SunLogger>> + Send + Sync>,
    ) -> Self {
        self.logger_factory = Some(f);
        self
    }

    #[inline]
    pub fn profiler_factory(
        mut self,
        f: Arc<dyn Fn(SunComm, &str) -> SunResult<Arc<dyn SunProfiler>> + Send + Sync>,
    ) -> Self {
        self.profiler_factory = Some(f);
        self
    }

    #[inline]
    pub fn default_err_handler(mut self, f: ErrHandlerFn) -> Self {
        self.default_err_handler = Some(f);
        self
    }

    #[inline]
    pub fn build(self) -> SunResult<SunContext> {
        SunContext::create_with(
            self.comm,
            self.config,
            self.logger_factory,
            self.profiler_factory,
            self.default_err_handler,
        )
    }
}

/// Main context object (`struct SUNContext_` equivalent).
pub struct SunContext {
    python: Option<Arc<dyn Send + Sync>>,
    logger: Option<Arc<dyn SunLogger>>,
    own_logger: bool,
    profiler: Option<Arc<dyn SunProfiler>>,
    own_profiler: bool,
    last_err: SunErrCode,
    err_handler: Option<Box<ErrHandlerNode>>,
    comm: SunComm,
    config: SunContextConfig,
}

impl SunContext {
    /// Create a context (equivalent to `SUNContext_Create`).
    #[inline]
    pub fn create(comm: SunComm) -> SunResult<Self> {
        SunContextBuilder::new(comm).build()
    }

    fn create_with(
        comm: SunComm,
        config: SunContextConfig,
        logger_factory: Option<Arc<dyn Fn(SunComm) -> SunResult<Arc<dyn SunLogger>> + Send + Sync>>,
        profiler_factory: Option<
            Arc<dyn Fn(SunComm, &str) -> SunResult<Arc<dyn SunProfiler>> + Send + Sync>,
        >,
        default_err_handler: Option<ErrHandlerFn>,
    ) -> SunResult<Self> {
        let logger = if let Some(f) = logger_factory {
            if config.logging_level > 0 {
                Some(f(if config.mpi_enabled { comm } else { 0 })?)
            } else {
                let lg = f(0)?;
                for setter in [
                    SunLogger::set_error_filename,
                    SunLogger::set_warning_filename,
                    SunLogger::set_info_filename,
                    SunLogger::set_debug_filename,
                ] {
                    setter(&*lg, "")?;
                }
                Some(lg)
            }
        } else {
            None
        };

        let profiler = if config.profiling_enabled && !config.caliper_enabled {
            profiler_factory
                .map(|f| f(comm, "SUNContext Default"))
                .transpose()?
        } else {
            None
        };

        let err_handler = default_err_handler.map(|f| {
            Box::new(ErrHandlerNode {
                f,
                user_data: None,
                previous: None,
            })
        });

        Ok(Self {
            python: None,
            own_logger: logger.is_some(),
            logger,
            own_profiler: profiler.is_some(),
            profiler,
            last_err: SunErrCode::Success,
            err_handler,
            comm,
            config,
        })
    }

    #[inline]
    pub fn get_last_error(&mut self) -> SunResult<SunErrCode> {
        let err = self.last_err;
        self.last_err = SunErrCode::Success;
        Ok(err)
    }

    #[inline]
    pub fn peek_last_error(&self) -> SunResult<SunErrCode> {
        Ok(self.last_err)
    }

    #[inline]
    pub fn push_err_handler(
        &mut self,
        err_fn: ErrHandlerFn,
        err_user_data: Option<Arc<dyn Send + Sync>>,
    ) -> SunResult<()> {
        let prev = self.err_handler.take();
        self.err_handler = Some(Box::new(ErrHandlerNode {
            f: err_fn,
            user_data: err_user_data,
            previous: prev,
        }));
        Ok(())
    }

    #[inline]
    pub fn pop_err_handler(&mut self) -> SunResult<()> {
        if let Some(mut top) = self.err_handler.take() {
            self.err_handler = top.previous.take();
        }
        Ok(())
    }

    #[inline]
    pub fn clear_err_handlers(&mut self) -> SunResult<()> {
        while self.err_handler.is_some() {
            self.pop_err_handler()?;
        }
        Ok(())
    }

    #[inline]
    pub fn get_profiler(&self) -> SunResult<Option<Arc<dyn SunProfiler>>> {
        Ok(self
            .config
            .profiling_enabled
            .then(|| self.profiler.clone())
            .flatten())
    }

    #[inline]
    pub fn set_profiler(&mut self, profiler: Option<Arc<dyn SunProfiler>>) -> SunResult<()> {
        if self.config.profiling_enabled {
            self.profiler = profiler;
            self.own_profiler = false;
        }
        Ok(())
    }

    #[inline]
    pub fn get_logger(&self) -> SunResult<Option<Arc<dyn SunLogger>>> {
        Ok(self.logger.clone())
    }

    #[inline]
    pub fn set_logger(&mut self, logger: Option<Arc<dyn SunLogger>>) -> SunResult<()> {
        self.logger = logger;
        self.own_logger = false;
        Ok(())
    }

    /// Explicit free equivalent (`SUNContext_Free`); also performed by `Drop`.
    pub fn free(&mut self) -> SunResult<()> {
        if self.config.profiling_enabled
            && !self.config.caliper_enabled
            && let Some(p) = &self.profiler
            && let Ok(v) = env::var("SUNPROFILER_PRINT")
            && v != "0"
        {
            if matches!(v.as_str(), "1" | "TRUE" | "stdout") {
                let mut out = std::io::stdout();
                p.print(&mut out)?;
            } else if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(v) {
                p.print(&mut f)?;
            }
        }

        if let Some(node) = &self.err_handler {
            let _ = (&node.f, &node.user_data);
        }

        self.clear_err_handlers()?;
        self.python = None;
        self.logger = None;
        self.profiler = None;
        Ok(())
    }

    #[inline]
    pub const fn comm(&self) -> SunComm {
        self.comm
    }
}

impl Drop for SunContext {
    #[inline]
    fn drop(&mut self) {
        let _ = self.free();
    }
}
