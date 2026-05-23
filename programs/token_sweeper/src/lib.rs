// ============================================================
// Token Sweeper Program — deploy on https://beta.solpg.io/
//
// Instructions (burn + close only — fees handled on the website):
//   1. close_spl_account       — close empty SPL token account, rent → owner
//   2. burn_spl_token          — burn SPL tokens + close account
//   3. close_token2022_account — close empty Token-2022 account, rent → owner
//   4. burn_token2022          — burn Token-2022 tokens + close account
//   5. burn_nft                — burn standard Metaplex NFT
//   6. burn_pnft               — burn Programmable NFT (pNFT)
//   7. burn_core_nft           — burn MPL Core NFT
//   8. burn_cnft               — burn Compressed NFT via Bubblegum
// ============================================================

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    self,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};
use anchor_spl::token::{self, Burn, CloseAccount, Mint, Token, TokenAccount};

declare_id!("5o8wsyECKtCwT72BNQJRxm1U5xDeVL5bD69WGxAFZSKB");

// External program IDs
const MPL_METADATA: Pubkey = solana_program::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const MPL_BUBBLEGUM: Pubkey = solana_program::pubkey!("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");
const MPL_CORE: Pubkey    = solana_program::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
const TOKEN_2022: Pubkey  = solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

#[program]
pub mod token_sweeper {
    use super::*;

    // ── 1. Close empty SPL token account — rent goes back to owner ─────────

    pub fn close_spl_account(ctx: Context<CloseSplAccount>) -> Result<()> {
        require_eq!(
            ctx.accounts.token_account.amount,
            0,
            SweepError::TokenAccountNotEmpty
        );

        token::close_account(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account:     ctx.accounts.token_account.to_account_info(),
                destination: ctx.accounts.owner.to_account_info(),
                authority:   ctx.accounts.owner.to_account_info(),
            },
        ))
    }

    // ── 2. Burn SPL tokens + close account ────────────────────────────────

    pub fn burn_spl_token(ctx: Context<BurnSplToken>, amount: u64) -> Result<()> {
        if amount > 0 {
            token::burn(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Burn {
                        mint:      ctx.accounts.mint.to_account_info(),
                        from:      ctx.accounts.token_account.to_account_info(),
                        authority: ctx.accounts.owner.to_account_info(),
                    },
                ),
                amount,
            )?;
        }

        token::close_account(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account:     ctx.accounts.token_account.to_account_info(),
                destination: ctx.accounts.owner.to_account_info(),
                authority:   ctx.accounts.owner.to_account_info(),
            },
        ))
    }

    // ── 3. Close empty Token-2022 account — rent goes back to owner ────────

    pub fn close_token2022_account(ctx: Context<CloseToken2022Account>) -> Result<()> {
        // CloseAccount discriminator = 9 (same index as SPL Token)
        invoke(
            &Instruction {
                program_id: TOKEN_2022,
                accounts: vec![
                    AccountMeta::new(ctx.accounts.token_account.key(), false),
                    AccountMeta::new(ctx.accounts.owner.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.owner.key(), true),
                ],
                data: vec![9u8],
            },
            &[
                ctx.accounts.token_account.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.owner.to_account_info(),
            ],
        ).map_err(Into::into)
    }

    // ── 4. Burn Token-2022 tokens + close account ─────────────────────────
    // `decimals` must match the mint's actual decimals (BurnChecked requirement)

    pub fn burn_token2022(
        ctx: Context<BurnToken2022>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        if amount > 0 {
            // BurnChecked discriminator = 25
            let mut burn_data = vec![25u8];
            burn_data.extend_from_slice(&amount.to_le_bytes());
            burn_data.push(decimals);

            invoke(
                &Instruction {
                    program_id: TOKEN_2022,
                    accounts: vec![
                        AccountMeta::new(ctx.accounts.token_account.key(), false),
                        AccountMeta::new(ctx.accounts.mint.key(), false),
                        AccountMeta::new_readonly(ctx.accounts.owner.key(), true),
                    ],
                    data: burn_data,
                },
                &[
                    ctx.accounts.token_account.to_account_info(),
                    ctx.accounts.mint.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                ],
            )?;
        }

        // CloseAccount discriminator = 9
        invoke(
            &Instruction {
                program_id: TOKEN_2022,
                accounts: vec![
                    AccountMeta::new(ctx.accounts.token_account.key(), false),
                    AccountMeta::new(ctx.accounts.owner.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.owner.key(), true),
                ],
                data: vec![9u8],
            },
            &[
                ctx.accounts.token_account.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.owner.to_account_info(),
            ],
        ).map_err(Into::into)
    }

    // ── 5. Burn standard Metaplex NFT ─────────────────────────────────────
    // BurnNft = instruction 29 in mpl-token-metadata.
    // remaining_accounts[0] = collection_metadata (writable, if verified collection)

    pub fn burn_nft<'info>(ctx: Context<'_, '_, '_, 'info, BurnNft<'info>>) -> Result<()> {
        let mut accounts = vec![
            AccountMeta::new(ctx.accounts.metadata.key(), false),
            AccountMeta::new(ctx.accounts.owner.key(), true),
            AccountMeta::new(ctx.accounts.mint.key(), false),
            AccountMeta::new(ctx.accounts.token_account.key(), false),
            AccountMeta::new(ctx.accounts.master_edition.key(), false),
            AccountMeta::new_readonly(ctx.accounts.spl_token_program.key(), false),
        ];
        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.token_account.to_account_info(),
            ctx.accounts.master_edition.to_account_info(),
            ctx.accounts.spl_token_program.to_account_info(),
        ];

        if let Some(col) = ctx.remaining_accounts.first() {
            accounts.push(AccountMeta::new(col.key(), false));
            account_infos.push(col.to_account_info());
        }

        invoke(
            &Instruction { program_id: MPL_METADATA, accounts, data: vec![29u8] },
            &account_infos,
        ).map_err(Into::into)
    }

    // ── 6. Burn Programmable NFT (pNFT) ───────────────────────────────────
    // BurnV1 = instruction 49, data = [49] ++ amount(u64 LE = 1)
    // remaining_accounts[0] = collection_metadata (writable, if verified collection)

    pub fn burn_pnft<'info>(ctx: Context<'_, '_, '_, 'info, BurnPnft<'info>>) -> Result<()> {
        let mut data = vec![49u8];
        data.extend_from_slice(&1u64.to_le_bytes());

        let mut accounts = vec![
            AccountMeta::new(ctx.accounts.metadata.key(), false),
            AccountMeta::new(ctx.accounts.owner.key(), true),
            AccountMeta::new(ctx.accounts.mint.key(), false),
            AccountMeta::new(ctx.accounts.token_account.key(), false),
            AccountMeta::new(ctx.accounts.edition.key(), false),
            AccountMeta::new(ctx.accounts.token_record.key(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(
                anchor_lang::solana_program::sysvar::instructions::ID,
                false,
            ),
            AccountMeta::new_readonly(ctx.accounts.spl_token_program.key(), false),
            AccountMeta::new_readonly(MPL_METADATA, false),
        ];
        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.token_account.to_account_info(),
            ctx.accounts.edition.to_account_info(),
            ctx.accounts.token_record.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.sysvar_instructions.to_account_info(),
            ctx.accounts.spl_token_program.to_account_info(),
            ctx.accounts.mpl_metadata_program.to_account_info(),
        ];

        if let Some(col) = ctx.remaining_accounts.first() {
            accounts.push(AccountMeta::new(col.key(), false));
            account_infos.push(col.to_account_info());
        }

        invoke(
            &Instruction { program_id: MPL_METADATA, accounts, data },
            &account_infos,
        ).map_err(Into::into)
    }

    // ── 7. Burn MPL Core NFT ──────────────────────────────────────────────
    // Single asset account holds everything — no mint/edition/ATA needed.
    // BurnV1 = byte 7, compressionProof = None (byte 0)
    // remaining_accounts[0] = collection (writable, if in a collection)
    // remaining_accounts[1] = log_wrapper / noop (read-only, optional)

    pub fn burn_core_nft<'info>(
        ctx: Context<'_, '_, '_, 'info, BurnCoreNft<'info>>,
    ) -> Result<()> {
        let data: Vec<u8> = vec![7u8, 0u8];

        let mut accounts = vec![
            AccountMeta::new(ctx.accounts.asset.key(), false),
            AccountMeta::new(ctx.accounts.owner.key(), true),
        ];
        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            ctx.accounts.asset.to_account_info(),
            ctx.accounts.owner.to_account_info(),
        ];

        for (i, acc) in ctx.remaining_accounts.iter().enumerate() {
            if i == 0 {
                accounts.push(AccountMeta::new(acc.key(), false)); // collection — writable
            } else {
                accounts.push(AccountMeta::new_readonly(acc.key(), false)); // log_wrapper
            }
            account_infos.push(acc.to_account_info());
        }

        // Core always needs System Program
        accounts.push(AccountMeta::new_readonly(anchor_lang::system_program::ID, false));
        account_infos.push(ctx.accounts.system_program.to_account_info());

        invoke(
            &Instruction { program_id: MPL_CORE, accounts, data },
            &account_infos,
        ).map_err(Into::into)
    }

    // ── 8. Burn Compressed NFT (cNFT) via Bubblegum ───────────────────────
    // Pass all Merkle proof nodes in remaining_accounts (read-only, root → leaf).
    // root/data_hash/creator_hash/nonce/index come from Helius getAssetProof.

    pub fn burn_cnft<'info>(
        ctx: Context<'_, '_, '_, 'info, BurnCnft<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        // Bubblegum Burn discriminator = sha256("global:burn")[..8]
        let discriminator: [u8; 8] = [116, 110, 29, 56, 107, 219, 42, 93];

        let mut data = discriminator.to_vec();
        data.extend_from_slice(&root);
        data.extend_from_slice(&data_hash);
        data.extend_from_slice(&creator_hash);
        data.extend_from_slice(&nonce.to_le_bytes());
        data.extend_from_slice(&index.to_le_bytes());

        let mut accounts = vec![
            AccountMeta::new(ctx.accounts.tree_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_delegate.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ];
        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            ctx.accounts.tree_authority.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_delegate.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        for proof_node in ctx.remaining_accounts.iter() {
            accounts.push(AccountMeta::new_readonly(proof_node.key(), false));
            account_infos.push(proof_node.to_account_info());
        }

        invoke(
            &Instruction { program_id: MPL_BUBBLEGUM, accounts, data },
            &account_infos,
        ).map_err(Into::into)
    }
}

// ── Account structs ────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct CloseSplAccount<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        token::authority = owner,
        constraint = token_account.amount == 0 @ SweepError::TokenAccountNotEmpty,
    )]
    pub token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BurnSplToken<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = owner)]
    pub token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseToken2022Account<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Token-2022 account — validated by CPI
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnToken2022<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: Token-2022 mint
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK: Token-2022 token account
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BurnNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: NFT mint
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK: Metadata PDA — seeds [b"metadata", MPL_METADATA, mint]
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    /// CHECK: ATA holding the NFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: Master Edition PDA — seeds [b"metadata", MPL_METADATA, mint, b"edition"]
    #[account(mut)]
    pub master_edition: AccountInfo<'info>,
    /// CHECK: SPL Token program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
    pub spl_token_program: AccountInfo<'info>,
    // remaining_accounts[0] = collection_metadata (optional, writable)
}

#[derive(Accounts)]
pub struct BurnPnft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: pNFT mint
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK: Metadata PDA
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    /// CHECK: Token account holding the pNFT
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: Master / Print edition PDA
    #[account(mut)]
    pub edition: AccountInfo<'info>,
    /// CHECK: Token Record PDA
    ///        seeds: [b"metadata", MPL_METADATA, mint, b"token_record", token_account]
    #[account(mut)]
    pub token_record: AccountInfo<'info>,
    /// CHECK: SPL Token program
    pub spl_token_program: AccountInfo<'info>,
    /// CHECK: Sysvar Instructions
    pub sysvar_instructions: AccountInfo<'info>,
    /// CHECK: MPL Token Metadata program
    pub mpl_metadata_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    // remaining_accounts[0] = collection_metadata (optional, writable)
}

#[derive(Accounts)]
pub struct BurnCoreNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: MPL Core asset account — owned by CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d
    #[account(mut)]
    pub asset: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    // remaining_accounts[0] = collection (writable, if NFT is in a collection)
    // remaining_accounts[1] = log_wrapper / noop (read-only, optional)
}

#[derive(Accounts)]
pub struct BurnCnft<'info> {
    #[account(mut)]
    pub leaf_owner: Signer<'info>,
    /// CHECK: Tree Authority PDA — seeds [merkle_tree] under MPL_BUBBLEGUM
    pub tree_authority: AccountInfo<'info>,
    /// CHECK: Leaf delegate (use leaf_owner if no separate delegate)
    pub leaf_delegate: AccountInfo<'info>,
    /// CHECK: SPL account-compression Merkle tree
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: SPL Noop program (noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV)
    pub log_wrapper: AccountInfo<'info>,
    /// CHECK: SPL Account Compression (cmtDvXumGCrqC1Age74AVPhSRVXJMd8PJS91L8KbNCK)
    pub compression_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    // remaining_accounts = proof nodes (all read-only, ordered root → leaf)
}

// ── Errors ────────────────────────────────────────────────────────────────

#[error_code]
pub enum SweepError {
    #[msg("Token account still holds tokens — burn them first")]
    TokenAccountNotEmpty,
}
