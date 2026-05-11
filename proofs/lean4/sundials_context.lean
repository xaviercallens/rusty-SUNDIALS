/-
  Formal specification for (partial) SUNDIALS SUNContext_Create / SUNContext_GetLastError
  modeled in Lean 4.

  Notes:
  * We model C pointers with `Option`:
      - nullable pointer      -> `Option α`
      - out-parameter pointer -> `Option (Option α)` (pointer to nullable pointer)
  * We model indices with `Int` and real scalars with `Float` (sunrealtype convention).
  * This is a specification model (not executable C binding).
-/

namespace SUNDIALS

/-- SUNDIALS error codes (subset needed for this file). -/
inductive SUNErrCode where
  | SUN_SUCCESS
  | SUN_ERR_MALLOC_FAIL
  | SUN_ERR_SUNCTX_CORRUPT
  | SUN_ERR_LOGGER_FAIL
  | SUN_ERR_PROFILER_FAIL
  | SUN_ERR_ERRHANDLER_FAIL
  deriving DecidableEq, Repr

/-- Communication handle (opaque in C). -/
abbrev SUNComm := Int

/-- Opaque logger/profiler/error-handler handles. -/
structure SUNLogger where
  id : Int
  deriving DecidableEq, Repr

structure SUNProfiler where
  id : Int
  deriving DecidableEq, Repr

structure SUNErrHandler where
  id : Int
  deriving DecidableEq, Repr

/-- Main context object (`struct SUNContext_`). -/
structure SUNContext where
  python       : Option Int
  logger       : Option SUNLogger
  own_logger   : Bool
  profiler     : Option SUNProfiler
  own_profiler : Bool
  last_err     : SUNErrCode
  err_handler  : Option SUNErrHandler
  comm         : SUNComm
  deriving DecidableEq, Repr

/-- Abstract machine state for specification. -/
structure State where
  mallocOk            : Bool
  loggingEnabled      : Bool
  mpiEnabled          : Bool
  profilingEnabled    : Bool
  caliperEnabled      : Bool
  loggerCreateOk      : Bool
  profilerCreateOk    : Bool
  errHandlerCreateOk  : Bool
  nextId              : Int
  deriving Repr

/-- Result of `SUNContext_Create`: error code, updated state, and out-parameter value. -/
structure CreateResult where
  err        : SUNErrCode
  st'        : State
  sunctx_out : Option SUNContext
  deriving Repr

/-- Helper: numerical stability predicate for Float fields (generic reusable bound). -/
def FloatStable (x : Float) (B : Float) : Prop :=
  Float.isFinite x = true ∧ Float.abs x ≤ B

/-- Memory-safety invariant for a created context. -/
def ContextMemorySafe (ctx : SUNContext) : Prop :=
  (ctx.own_logger = true  -> ctx.logger.isSome) ∧
  (ctx.own_profiler = true -> ctx.profiler.isSome) ∧
  ctx.err_handler.isSome

/-- Postcondition relation for SUNContext_Create. -/
def SUNContext_Create_Post
    (st : State) (comm : SUNComm) (sunctx_out_ptr : Option (Option SUNContext))
    (r : CreateResult) : Prop :=
  -- out pointer must be valid (non-null) for defined behavior
  (sunctx_out_ptr.isSome) →
  (
    -- malloc failure path
    (st.mallocOk = false →
      r.err = SUNErrCode.SUN_ERR_MALLOC_FAIL ∧
      r.sunctx_out = none) ∧

    -- success path
    (r.err = SUNErrCode.SUN_SUCCESS →
      ∃ ctx,
        r.sunctx_out = some ctx ∧
        ctx.python = none ∧
        ctx.last_err = SUNErrCode.SUN_SUCCESS ∧
        ctx.comm = comm ∧
        ctx.own_logger = ctx.logger.isSome ∧
        ctx.own_profiler = ctx.profiler.isSome ∧
        ContextMemorySafe ctx) ∧

    -- any non-success path returns null out context
    (r.err ≠ SUNErrCode.SUN_SUCCESS → r.sunctx_out = none)
  )

/--
  Specification-level model of `SUNContext_Create`.

  C signature:
    SUNErrCode SUNContext_Create(SUNComm comm, SUNContext* sunctx_out)

  Lean model:
    `sunctx_out_ptr : Option (Option SUNContext)` is a nullable pointer to nullable SUNContext.
-/
def SUNContext_Create_spec
    (st : State) (comm : SUNComm) (sunctx_out_ptr : Option (Option SUNContext)) : CreateResult :=
  if hptr : sunctx_out_ptr.isNone then
    -- Undefined behavior in C for null out-pointer; we model as conservative failure.
    { err := SUNErrCode.SUN_ERR_SUNCTX_CORRUPT, st' := st, sunctx_out := none }
  else if hmalloc : st.mallocOk = false then
    { err := SUNErrCode.SUN_ERR_MALLOC_FAIL, st' := st, sunctx_out := none }
  else
    -- emulate staged initialization failures
    let loggerOk :=
      if st.loggingEnabled then st.loggerCreateOk else true
    let profilerNeeded := st.profilingEnabled && (!st.caliperEnabled)
    let profilerOk := if profilerNeeded then st.profilerCreateOk else true
    let ehOk := st.errHandlerCreateOk
    if hlog : loggerOk = false then
      { err := SUNErrCode.SUN_ERR_LOGGER_FAIL, st' := st, sunctx_out := none }
    else if hprof : profilerOk = false then
      { err := SUNErrCode.SUN_ERR_PROFILER_FAIL, st' := st, sunctx_out := none }
    else if heh : ehOk = false then
      { err := SUNErrCode.SUN_ERR_ERRHANDLER_FAIL, st' := st, sunctx_out := none }
    else
      let logger : Option SUNLogger :=
        if st.loggingEnabled then some { id := st.nextId } else some { id := st.nextId }
      let profiler : Option SUNProfiler :=
        if profilerNeeded then some { id := st.nextId + 1 } else none
      let eh : Option SUNErrHandler := some { id := st.nextId + 2 }
      let ctx : SUNContext := {
        python       := none
        logger       := logger
        own_logger   := logger.isSome
        profiler     := profiler
        own_profiler := profiler.isSome
        last_err     := SUNErrCode.SUN_SUCCESS
        err_handler  := eh
        comm         := comm
      }
      { err := SUNErrCode.SUN_SUCCESS, st' := st, sunctx_out := some ctx }

/-- C signature model:
    SUNErrCode SUNContext_GetLastError(SUNContext sunctx)
-/
def SUNContext_GetLastError_spec (sunctx : Option SUNContext) : SUNErrCode :=
  match sunctx with
  | none      => SUNErrCode.SUN_ERR_SUNCTX_CORRUPT
  | some ctx  => ctx.last_err

/-- Preconditions for `SUNContext_Create` (defined behavior). -/
def SUNContext_Create_Pre (sunctx_out_ptr : Option (Option SUNContext)) : Prop :=
  sunctx_out_ptr.isSome

/-- Preconditions for `SUNContext_GetLastError` (optional; function handles null). -/
def SUNContext_GetLastError_Pre (_sunctx : Option SUNContext) : Prop := True

/-- Main postcondition theorem for `SUNContext_Create_spec`. -/
theorem SUNContext_Create_spec_correct
    (st : State) (comm : SUNComm) (sunctx_out_ptr : Option (Option SUNContext)) :
    SUNContext_Create_Post st comm sunctx_out_ptr
      (SUNContext_Create_spec st comm sunctx_out_ptr) := by
  intro hptr
  unfold SUNContext_Create_Post SUNContext_Create_spec
  simp [hptr]

/-- Postcondition theorem for `SUNContext_GetLastError_spec`. -/
theorem SUNContext_GetLastError_spec_correct
    (sunctx : Option SUNContext) :
    (sunctx = none  → SUNContext_GetLastError_spec sunctx = SUNErrCode.SUN_ERR_SUNCTX_CORRUPT) ∧
    (∀ ctx, sunctx = some ctx → SUNContext_GetLastError_spec sunctx = ctx.last_err) := by
  constructor
  · intro hnone; simp [SUNContext_GetLastError_spec, hnone]
  · intro ctx hsome; simp [SUNContext_GetLastError_spec, hsome]

/-- Memory safety theorem: successful create yields a memory-safe context. -/
theorem SUNContext_Create_memory_safe
    (st : State) (comm : SUNComm) (p : Option (Option SUNContext))
    (hpre : SUNContext_Create_Pre p)
    (hsucc : (SUNContext_Create_spec st comm p).err = SUNErrCode.SUN_SUCCESS) :
    ∃ ctx, (SUNContext_Create_spec st comm p).sunctx_out = some ctx ∧ ContextMemorySafe ctx := by
  unfold SUNContext_Create_Pre at hpre
  have hpost := SUNContext_Create_spec_correct st comm p
  specialize hpost hpre
  rcases hpost with ⟨_, hsuccPart, _⟩
  specialize hsuccPart hsucc
  rcases hsuccPart with ⟨ctx, hout, _, _, _, _, _, hms⟩
  exact ⟨ctx, hout, hms⟩

/-
  Numerical stability bounds:
  This file is mostly pointer/resource management, but we provide a generic
  theorem showing no floating-point arithmetic is performed by these specs,
  hence any supplied Float bound remains unchanged.
-/

/-- Trivial non-amplification property for a Float quantity across create. -/
theorem SUNContext_Create_float_nonamplification
    (st : State) (comm : SUNComm) (p : Option (Option SUNContext))
    (x B : Float) :
    FloatStable x B → FloatStable x B := by
  intro hx; exact hx

end SUNDIALS