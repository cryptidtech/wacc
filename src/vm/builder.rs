// SPDX-License-Identifier: Apache-2.0
use crate::{
    error::VmError,
    security::SecurityLimits,
    types::FuelAmount,
    vm::runtime::default_runtime,
    vm::{ComponentInstance, ScriptKind},
    Context, Error, Instance, PreparedModule, Runtime,
};
use std::sync::Arc;
use wasmtime::Store;

/// Builder type for constructing WACC VM instances
///
/// The builder pattern provides a fluent API for creating VM instances
/// with validation and sensible defaults.
///
/// # Security
///
/// By default, the builder enforces production security limits including:
/// - Fuel: 1,000,000 operations (prevents infinite loops)
/// - Check count: 256 max (limits crypto operations)
/// - Memory: 4 MB (prevents exhaustion)
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// # use wacc::{Builder, Context, types::{CheckCount, ContextPath}, storage::{Pairs, Stack}, Value};
/// # use std::collections::BTreeMap;
/// # use wasmtime::StoreLimitsBuilder;
/// # #[derive(Default)]
/// # struct DummyPairs;
/// # impl Pairs for DummyPairs {
/// #     fn get(&self, _: &str) -> Option<Value> { None }
/// #     fn put(&mut self, _: &str, _: &Value) -> Option<Value> { None }
/// # }
/// # #[derive(Default)]
/// # struct DummyStack;
/// # impl Stack for DummyStack {
/// #     fn push(&mut self, _: Value) {}
/// #     fn pop(&mut self) -> Option<Value> { None }
/// #     fn top(&self) -> Option<Value> { None }
/// #     fn peek(&self, _: usize) -> Option<Value> { None }
/// #     fn len(&self) -> usize { 0 }
/// #     fn is_empty(&self) -> bool { true }
/// # }
/// # let wasm_bytes = vec![0u8; 8];
/// let context = Context {
///     current: Box::new(DummyPairs),
///     proposed: Box::new(DummyPairs),
///     pstack: Box::new(DummyStack),
///     rstack: Box::new(DummyStack),
///     check_count: CheckCount::zero(),
///     write_idx: 0,
///     context: ContextPath::root(),
///     log: Vec::default(),
///     limiter: StoreLimitsBuilder::new().build(),
/// };
///
/// let mut instance = Builder::new()
///     .with_context(context)
///     .with_bytes(&wasm_bytes)
///     .try_build()?;
///
/// let result = instance.run("main")?;
/// # Ok::<(), wacc::Error>(())
/// ```
///
/// ## Strict Mode for Untrusted Code
///
/// ```rust,no_run
/// # use wacc::{Builder, Context, types::{CheckCount, ContextPath}};
/// # use wacc::storage::{Pairs, Stack};
/// # use wasmtime::StoreLimitsBuilder;
/// # let context = Context {
/// #     current: unimplemented!(),
/// #     proposed: unimplemented!(),
/// #     pstack: unimplemented!(),
/// #     rstack: unimplemented!(),
/// #     check_count: CheckCount::zero(),
/// #     write_idx: 0,
/// #     context: ContextPath::root(),
/// #     log: Vec::default(),
/// #     limiter: StoreLimitsBuilder::new().build(),
/// # };
/// # let untrusted_wasm = &[0u8; 8];
/// let instance = Builder::new_strict()  // Lower limits
///     .with_context(context)
///     .with_bytes(untrusted_wasm)
///     .try_build()?;
/// # Ok::<(), wacc::Error>(())
/// ```
///
/// ## Custom Security Configuration
///
/// ```rust,no_run
/// # use wacc::{Builder, Context, SecurityLimits, types::{CheckCount, ContextPath, FuelAmount}};
/// # use wacc::storage::{Pairs, Stack};
/// # use wasmtime::StoreLimitsBuilder;
/// # let context = Context {
/// #     current: unimplemented!(),
/// #     proposed: unimplemented!(),
/// #     pstack: unimplemented!(),
/// #     rstack: unimplemented!(),
/// #     check_count: CheckCount::zero(),
/// #     write_idx: 0,
/// #     context: ContextPath::root(),
/// #     log: Vec::default(),
/// #     limiter: StoreLimitsBuilder::new().build(),
/// # };
/// # let wasm_bytes = &[0u8; 8];
/// let custom_limits = SecurityLimits {
///     max_fuel: FuelAmount::new(500_000),
///     ..SecurityLimits::PRODUCTION
/// };
///
/// let instance = Builder::new()
///     .with_security_limits(custom_limits)
///     .with_context(context)
///     .with_bytes(wasm_bytes)
///     .try_build()?;
/// # Ok::<(), wacc::Error>(())
/// ```
pub struct Builder {
    fuel: Option<FuelAmount>,
    bytes: Vec<u8>,
    context: Option<Context>,
    security_limits: SecurityLimits,
    runtime: Option<Arc<Runtime>>,
    prepared_module: Option<PreparedModule>,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    /// Creates a new builder with production security limits
    ///
    /// For untrusted code, consider using `new_strict()` instead.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fuel: Some(FuelAmount::default()),
            bytes: Vec::default(),
            context: None,
            security_limits: SecurityLimits::PRODUCTION,
            runtime: None,
            prepared_module: None,
        }
    }

    /// Creates a new builder with strict security limits for untrusted code
    ///
    /// Use this when executing WASM code from untrusted sources.
    #[must_use]
    pub fn new_strict() -> Self {
        Self {
            fuel: Some(FuelAmount::new(100_000)),
            bytes: Vec::default(),
            context: None,
            security_limits: SecurityLimits::STRICT,
            runtime: None,
            prepared_module: None,
        }
    }

    /// Creates a new builder with development limits (more permissive)
    ///
    /// Use this for development and testing only.
    #[must_use]
    pub fn new_dev() -> Self {
        Self {
            fuel: Some(FuelAmount::new(10 * FuelAmount::DEFAULT)),
            bytes: Vec::default(),
            context: None,
            security_limits: SecurityLimits::DEVELOPMENT,
            runtime: None,
            prepared_module: None,
        }
    }

    /// Sets custom security limits
    #[must_use]
    pub const fn with_security_limits(mut self, limits: SecurityLimits) -> Self {
        self.security_limits = limits;
        if self.fuel.is_none() {
            self.fuel = Some(limits.max_fuel);
        }
        self
    }

    /// Enables the use of fuel and establishes the fuel limit for the execution
    #[must_use]
    pub const fn with_fuel(mut self, fuel: FuelAmount) -> Self {
        self.fuel = Some(fuel);
        self
    }

    /// Initializes the [`Instance`] with the bytes to execute
    pub fn with_bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.bytes = bytes.as_ref().to_vec();
        self
    }

    /// Initializes the bytes to execute as a WASM component
    ///
    /// The bytes are compiled and instantiated through
    /// [`Self::try_build_component()`]. Feeding detectable module bytes here
    /// makes [`Self::try_build_component()`] fail with
    /// [`VmError::ScriptKindMismatch`], mirroring the guard that
    /// [`Self::try_build()`] applies to detectable component bytes.
    pub fn with_component_bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.bytes = bytes.as_ref().to_vec();
        self
    }

    /// Add the context for the application state
    #[must_use]
    pub fn with_context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    /// Uses a caller-provided runtime instead of the process-wide default.
    ///
    /// This is useful for controlling cache lifetime or isolating tenants.
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Uses bytecode compiled ahead of repeated instance construction.
    #[must_use]
    pub fn with_prepared_module(mut self, prepared: PreparedModule) -> Self {
        self.runtime = Some(Arc::clone(&prepared.runtime));
        self.prepared_module = Some(prepared);
        self
    }

    /// Tries to build the [`Instance`] from the builder configuration
    ///
    /// This method enforces security limits and validates all configuration.
    ///
    /// Detectable WASM component bytes are rejected with
    /// [`VmError::ScriptKindMismatch`]; only [`Self::try_build_component()`]
    /// executes components.
    pub fn try_build(mut self) -> Result<Instance, Error> {
        self.reject_kind_mismatch(ScriptKind::Module)?;

        let fuel = self.validated_fuel()?;

        let (runtime, module) = if let Some(prepared) = self.prepared_module.take() {
            if self
                .runtime
                .as_ref()
                .is_some_and(|runtime| !Arc::ptr_eq(runtime, &prepared.runtime))
            {
                return Err(Error::custom(
                    &"prepared module belongs to a different runtime",
                ));
            }
            (prepared.runtime, prepared.module)
        } else {
            let runtime = match self.runtime.take() {
                Some(runtime) => runtime,
                None => default_runtime()?,
            };
            let module = runtime.compile(&self.bytes)?;
            (runtime, module)
        };

        let context = self.required_context()?;

        // configure the store with fuel (now mandatory for security)
        let mut store = Store::new(runtime.engine(), context);
        store
            .set_fuel(fuel.as_u64())
            .map_err(Error::from_wasmtime)?;

        // configure the limiter
        store.limiter(|state| &mut state.limiter);

        // build the instance
        Ok(Instance {
            linker: runtime.linker(),
            module: module.as_ref().clone(),
            store,
        })
    }

    /// Tries to build the [`ComponentInstance`] from the builder configuration
    ///
    /// Mirrors [`Self::try_build()`]: the same fuel validation, context
    /// requirements, security limits, and store setup apply. The component is
    /// instantiated against the runtime's component linker, which provides the
    /// typed `cryptid:wacc/host@1.0.0` interface for both script worlds.
    ///
    /// A caller-provided [`PreparedModule`] does not apply here:
    /// [`PreparedModule`] carries compiled core modules only, so this method
    /// always compiles `self.bytes`.
    ///
    /// Detectable core-module bytes are rejected with
    /// [`VmError::ScriptKindMismatch`]; only [`Self::try_build()`] executes
    /// modules.
    pub fn try_build_component(mut self) -> Result<ComponentInstance, Error> {
        self.reject_kind_mismatch(ScriptKind::Component)?;

        let fuel = self.validated_fuel()?;

        let runtime = match self.runtime.take() {
            Some(runtime) => runtime,
            None => default_runtime()?,
        };
        let component = runtime.compile_component(&self.bytes)?;

        let context = self.required_context()?;

        // configure the store with fuel
        let mut store = Store::new(runtime.engine(), context);
        store
            .set_fuel(fuel.as_u64())
            .map_err(Error::from_wasmtime)?;

        // configure the limiter
        store.limiter(|state| &mut state.limiter);

        // instantiate the component and bind the instance
        let instance = runtime
            .component_linker()
            .instantiate(&mut store, component.as_ref())
            .map_err(|e| VmError::InstantiationError {
                message: format!("failed to instantiate WASM component: {e}"),
            })?;

        Ok(ComponentInstance::new(instance, store))
    }

    /// Returns the fuel to install, validated against the security limits
    fn validated_fuel(&self) -> Result<FuelAmount, Error> {
        match self.fuel {
            Some(f) => {
                if f.as_u64() > self.security_limits.max_fuel.as_u64() {
                    return Err(Error::custom(&format!(
                        "fuel {} exceeds security limit {}",
                        f, self.security_limits.max_fuel
                    )));
                }
                Ok(f)
            }
            None => {
                // Default to security limits fuel
                Ok(self.security_limits.max_fuel)
            }
        }
    }

    /// Returns the context required by every build path, failing otherwise
    fn required_context(&mut self) -> Result<Context, Error> {
        self.context.take().ok_or_else(|| {
            VmError::MissingContext {
                context: "Builder requires context to be set via with_context()".to_string(),
            }
            .into()
        })
    }

    /// Rejects input whose detected script kind does not match the kind the
    /// targeted build path executes
    fn reject_kind_mismatch(&self, expected: ScriptKind) -> Result<(), Error> {
        match ScriptKind::detect(&self.bytes) {
            Some(actual) if actual != expected => {
                Err(VmError::ScriptKindMismatch { expected, actual }.into())
            }
            _ => Ok(()),
        }
    }
}
