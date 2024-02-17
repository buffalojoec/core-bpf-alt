# Address Lookup Table Core BPF Implementation

This PR introduces the Address Lookup Table program reimplemented to Core BPF.

Some notable points of contention with this implementation are defined below.

### Feature `relax_authority_signer_check_for_lookup_table_creation`

Feature
[`FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap`](https://github.com/solana-labs/solana/issues/27205)
should be activated on mainnet-beta before Address Lookup Table is ported to
Core BPF, since this will simplify the program's interface. It's next up in the
queue.

This feature ID decorates all call sites which can be removed from the Core BPF
implementation once activated. They include:

- Instruction for creating a lookup table "signed".
- Processor checks for authority signatures on create.
- Tests ensuring authority checks are conducted.
- The Rust feature flag I'm using to mock this feature ID:
  `relax-authority-checks-disabled`.

### Error Codes

Since built-in programs throw `InstructionError` and BPF programs throw
`ProgramError`, we simply need to use the corresponding variants from
`ProgramError`, which map to the same error code. See this in action:

<https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/sdk/program/src/program_error.rs#L289>

However, there are a few missing variants from `ProgramError` that are present
in `InstructionError`. I've added these variants in this PR.

<https://github.com/solana-labs/solana/pull/35113>

We simply have to wait until those errors are available with the next release.

### Program-Test

`solana-program-test` will not overwrite a built-in if the BPF program you've
provided shares the same address as an existing built-in.

Technically this is not an issue with `solana-program-test` but with the
runtime.

I'm in the process of adding the Core BPF migration codepath to the runtime,
which will allow for built-ins to be manipulated, rather than a static
immutable list.

Until we can use `solana-program-test` to test Core BPF programs in place of
their built-in counterparts, we have to use a different address to test.

See [`lib.rs`](./src/lib.rs).