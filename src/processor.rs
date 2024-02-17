//! Program processor

#[cfg(not(feature = "relax-authority-checks-disabled"))]
use crate::check_id;
use {
    crate::{
        instruction::ProgramInstruction as AddressLookupTableInstruction,
        state::{
            AddressLookupTable, ProgramState, LOOKUP_TABLE_MAX_ADDRESSES, LOOKUP_TABLE_META_SIZE,
        },
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::{Clock, Slot},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
};

fn process_create_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    untrusted_recent_slot: Slot,
    bump_seed: u8,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;
    let payer_info = next_account_info(accounts_iter)?;
    let system_program_info = next_account_info(accounts_iter)?;

    // Feature "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
    // (relax_authority_signer_check_for_lookup_table_creation) is enabled on
    // only testnet and devnet, but not mainnet-beta.
    // - Testnet:       Epoch 586
    // - Devnet:        Epoch 591
    // - Mainnet-Beta:  Inactive
    //
    // At the time of this writing, this feature is first on the list for the
    // Mainnet-Beta Feature Gate Activation Schedule.
    // https://github.com/solana-labs/solana/wiki/Feature-Gate-Activation-Schedule
    //
    // It's my recommendation that we wait until after this feature is
    // activated on Mainnet-Beta before migrating Address Lookup Table to BPF,
    // removing these checks beforehand.
    //
    // Alternatively, we can fall back to this proposal if the feature is not
    // activated by the time we want to migrate to BPF.
    // https://github.com/solana-foundation/solana-improvement-documents/pull/99
    //
    // In the meantime, in order to fully test the BPF version of the program,
    // I've gated this functionality behind a feature flag.
    // `relax-authority-checks-disabled` should be provided to `cargo test-sbf`
    // in order to run the tests that depend on the checks being present, which
    // will be disabled once the feature is activated on Mainnet-Beta.
    //
    // Check not required after "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
    // is activated on mainnet-beta.
    #[cfg(feature = "relax-authority-checks-disabled")]
    if !lookup_table_info.data_is_empty() {
        msg!("Table account must not be allocated");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Check not required after "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
    // is activated on mainnet-beta.
    #[cfg(feature = "relax-authority-checks-disabled")]
    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !payer_info.is_signer {
        msg!("Payer account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // TODO: `SlotHashes` is not available to BPF programs.
    // Ideally we can introduce a truncated version of `SlotHashes` that only
    // contains the most recent slot hashes.
    // let derivation_slot = {
    //     let slot_hashes = <SlotHashes as Sysvar>::get()?;
    //     if slot_hashes.get(&untrusted_recent_slot).is_some() {
    //         Ok(untrusted_recent_slot)
    //     } else {
    //         msg!("{} is not a recent slot", untrusted_recent_slot);
    //         Err(ProgramError::InvalidInstructionData)
    //     }
    // }?;
    //
    // Fake check!
    let derivation_slot = {
        if untrusted_recent_slot == 123 {
            Ok(untrusted_recent_slot)
        } else {
            msg!("{} is not a recent slot", untrusted_recent_slot);
            Err(ProgramError::InvalidInstructionData)
        }
    }?;

    // Use a derived address to ensure that an address table can never be
    // initialized more than once at the same address.
    let derived_table_key = Pubkey::create_program_address(
        &[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed],
        ],
        program_id,
    )?;

    if lookup_table_info.key != &derived_table_key {
        msg!(
            "Table address must match derived address: {}",
            derived_table_key
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Check not required after "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
    // is activated on mainnet-beta.
    #[cfg(not(feature = "relax-authority-checks-disabled"))]
    if check_id(lookup_table_info.owner) {
        return Ok(());
    }

    let lookup_table_data_len = LOOKUP_TABLE_META_SIZE;
    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(lookup_table_data_len)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(payer_info.key, lookup_table_info.key, required_lamports),
            &[
                payer_info.clone(),
                lookup_table_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    invoke_signed(
        &system_instruction::allocate(lookup_table_info.key, lookup_table_data_len as u64),
        &[lookup_table_info.clone(), system_program_info.clone()],
        &[&[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed], // We _could_ eliminate this from the instruction
        ]],
    )?;

    invoke_signed(
        &system_instruction::assign(lookup_table_info.key, program_id),
        &[lookup_table_info.clone(), system_program_info.clone()],
        &[&[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed], // We _could_ eliminate this from the instruction
        ]],
    )?;

    // TODO: Re-work some of this serialization logic
    ProgramState::serialize_new_lookup_table(
        *lookup_table_info.try_borrow_mut_data()?,
        authority_info.key,
    )?;

    Ok(())
}

fn process_freeze_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut lookup_table_meta = {
        // Scope the borrow
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        // TODO: Re-work some of this serialization logic
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is already frozen");
            // TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Deactivated tables cannot be frozen");
            return Err(ProgramError::InvalidArgument);
        }
        if lookup_table.addresses.is_empty() {
            msg!("Empty lookup tables cannot be frozen");
            return Err(ProgramError::InvalidInstructionData);
        }

        lookup_table.meta
    };

    // TODO: Re-work some of this serialization logic
    lookup_table_meta.authority = None;
    AddressLookupTable::overwrite_meta_data(
        *lookup_table_info.try_borrow_mut_data()?,
        lookup_table_meta,
    )?;

    Ok(())
}

fn process_extend_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_addresses: Vec<Pubkey>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (lookup_table_meta, lookup_table_addresses, new_table_data_len) = {
        let lookup_table_data = lookup_table_info.try_borrow_mut_data()?;
        // TODO: Re-work some of this serialization logic
        let mut lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            // TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Deactivated tables cannot be extended");
            return Err(ProgramError::InvalidArgument);
        }
        if lookup_table.addresses.len() >= LOOKUP_TABLE_MAX_ADDRESSES {
            msg!("Lookup table is full and cannot contain more addresses");
            return Err(ProgramError::InvalidArgument);
        }

        if new_addresses.is_empty() {
            msg!("Must extend with at least one address");
            return Err(ProgramError::InvalidInstructionData);
        }

        let new_table_addresses_len = lookup_table
            .addresses
            .len()
            .saturating_add(new_addresses.len());
        if new_table_addresses_len > LOOKUP_TABLE_MAX_ADDRESSES {
            msg!(
                "Extended lookup table length {} would exceed max capacity of {}",
                new_table_addresses_len,
                LOOKUP_TABLE_MAX_ADDRESSES,
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        let clock = <Clock as Sysvar>::get()?;
        if clock.slot != lookup_table.meta.last_extended_slot {
            lookup_table.meta.last_extended_slot = clock.slot;
            lookup_table.meta.last_extended_slot_start_index =
                u8::try_from(lookup_table.addresses.len()).map_err(|_| {
                    // This is impossible as long as the length of new_addresses
                    // is non-zero and LOOKUP_TABLE_MAX_ADDRESSES == u8::MAX + 1.
                    ProgramError::InvalidAccountData
                })?;
        }

        let new_table_data_len = LOOKUP_TABLE_META_SIZE
            .checked_add(new_table_addresses_len.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let lookup_table_meta = lookup_table.meta;
        let mut lookup_table_addresses = lookup_table.addresses.to_vec();
        for new_address in new_addresses {
            lookup_table_addresses.push(new_address);
        }

        (
            lookup_table_meta,
            lookup_table_addresses,
            new_table_data_len,
        )
    };

    // TODO: Re-work some of this serialization logic
    AddressLookupTable::overwrite_meta_data(
        *lookup_table_info.try_borrow_mut_data()?,
        lookup_table_meta,
    )?;

    // TODO: Re-work some of this serialization logic
    lookup_table_info.realloc(new_table_data_len, false)?;
    AddressLookupTable::overwrite_addresses(
        *lookup_table_info.try_borrow_mut_data()?,
        lookup_table_addresses.as_slice(),
    )?;

    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(new_table_data_len)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    if required_lamports > 0 {
        let payer_info = next_account_info(accounts_iter)?;
        let _system_program_info = next_account_info(accounts_iter)?;

        if !payer_info.is_signer {
            msg!("Payer account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        invoke(
            &system_instruction::transfer(payer_info.key, lookup_table_info.key, required_lamports),
            &[payer_info.clone(), lookup_table_info.clone()],
        )?;
    }

    Ok(())
}

fn process_deactivate_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut lookup_table_meta = {
        // Scope the borrow
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        // TODO: Re-work some of this serialization logic
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            // TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Lookup table is already deactivated");
            return Err(ProgramError::InvalidArgument);
        }

        lookup_table.meta
    };

    let clock = <Clock as Sysvar>::get()?;
    lookup_table_meta.deactivation_slot = clock.slot;

    // TODO: Re-work some of this serialization logic
    AddressLookupTable::overwrite_meta_data(
        *lookup_table_info.try_borrow_mut_data()?,
        lookup_table_meta,
    )?;

    Ok(())
}

fn process_close_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;
    let recipient_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Here the legacy built-in version of ALT fallibly checks to ensure the
    // number of instruction accounts is 3.
    // It also checks that the recipient account is not the same as the lookup
    // table account.
    // The built-in does this by specifically checking the account keys at
    // their respective indices in the instruction context.
    // In BPF, we can just compare the addresses directly.
    if lookup_table_info.key == recipient_info.key {
        msg!("Lookup table cannot be the recipient of reclaimed lamports");
        return Err(ProgramError::InvalidArgument);
    }

    {
        // Scope the borrow
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        // TODO: Re-work some of this serialization logic
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            // TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
    }

    let _clock = <Clock as Sysvar>::get()?;
    // TODO: `SlotHashes` is not available to BPF programs.
    // Ideally we can introduce a truncated version of `SlotHashes` that only
    // contains the most recent slot hashes.
    // let slot_hashes = <SlotHashes as Sysvar>::get()?;

    // match lookup_table.meta.status(clock.slot, &slot_hashes) {
    //     LookupTableStatus::Activated => {
    //         msg!("Lookup table is not deactivated");
    //         Err(ProgramError::InvalidArgument)
    //     },
    //     LookupTableStatus::Deactivating { remaining_blocks } => {
    //         msg!(
    //             "Table cannot be closed until it's fully deactivated in {}
    // blocks",             remaining_blocks
    //         );
    //         Err(ProgramError::InvalidArgument)
    //     },
    //     LookupTableStatus::Deactivated => Ok(()),
    // }?;

    let new_recipient_lamports = lookup_table_info
        .lamports()
        .checked_add(recipient_info.lamports())
        .ok_or::<ProgramError>(ProgramError::ArithmeticOverflow)?;

    **lookup_table_info.try_borrow_mut_lamports()? = 0;
    **recipient_info.try_borrow_mut_lamports()? = new_recipient_lamports;

    lookup_table_info.realloc(0, true)?;

    Ok(())
}

/// Processes an `AddressLookupTableInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    // TODO: The legacy built-in version of ALT deserializes the input with
    // `limited_deserialize`.
    // `limited_deserialize` may offer some performance benefits, but the
    // resulting error is still `InstructionError::InvalidInstructionData`, so
    // we have ABI compatibility here.
    let instruction =
        bincode::deserialize(input).map_err(|_| ProgramError::InvalidInstructionData)?;
    match instruction {
        AddressLookupTableInstruction::CreateLookupTable {
            recent_slot,
            bump_seed,
        } => {
            msg!("Instruction: CreateLookupTable");
            process_create_lookup_table(program_id, accounts, recent_slot, bump_seed)
        }
        AddressLookupTableInstruction::FreezeLookupTable => {
            msg!("Instruction: FreezeLookupTable");
            process_freeze_lookup_table(program_id, accounts)
        }
        AddressLookupTableInstruction::ExtendLookupTable { new_addresses } => {
            msg!("Instruction: ExtendLookupTable");
            process_extend_lookup_table(program_id, accounts, new_addresses)
        }
        AddressLookupTableInstruction::DeactivateLookupTable => {
            msg!("Instruction: DeactivateLookupTable");
            process_deactivate_lookup_table(program_id, accounts)
        }
        AddressLookupTableInstruction::CloseLookupTable => {
            msg!("Instruction: CloseLookupTable");
            process_close_lookup_table(program_id, accounts)
        }
    }
}
