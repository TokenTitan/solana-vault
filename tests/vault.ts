import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Vault } from "../target/types/vault";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import assert from "assert";

describe("vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Vault as Program<Vault>;
  const authority = provider.wallet as anchor.Wallet;

  let mint: PublicKey;
  let authorityTokenAccount: PublicKey;
  let vaultStatePda: PublicKey;
  let vaultTokenAccountPda: PublicKey;

  const DEPOSIT_AMOUNT = new anchor.BN(1_000_000);
  const HALF_DEPOSIT_AMOUNT = new anchor.BN(500_000);
  const TOKEN_DECIMALS = 6;

  before(async () => {
    mint = await createMint(
      provider.connection,
      authority.payer,
      authority.publicKey,
      null,
      TOKEN_DECIMALS
    );

    authorityTokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      authority.payer,
      mint,
      authority.publicKey
    );

    await mintTo(
      provider.connection,
      authority.payer,
      mint,
      authorityTokenAccount,
      authority.publicKey,
      10_000_000
    );

    [vaultStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), authority.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    );

    [vaultTokenAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), authority.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    );
  });

  it("initializes the vault", async () => {
    await program.methods
      .initializeVault()
      .accountsPartial({
        authority: authority.publicKey,
        mint,
        vaultState: vaultStatePda,
        vaultTokenAccount: vaultTokenAccountPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const vaultState = await program.account.vaultState.fetch(vaultStatePda);
    assert.equal(
      vaultState.authority.toBase58(),
      authority.publicKey.toBase58()
    );
    assert.equal(vaultState.mint.toBase58(), mint.toBase58());
    assert.equal(vaultState.deposited.toString(), "0");
  });

  it("deposits tokens into the vault", async () => {
    await program.methods
      .deposit(DEPOSIT_AMOUNT)
      .accountsPartial({
        authority: authority.publicKey,
        mint,
        vaultState: vaultStatePda,
        authorityTokenAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultState = await program.account.vaultState.fetch(vaultStatePda);
    assert.equal(vaultState.deposited.toString(), DEPOSIT_AMOUNT.toString());

    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccountPda
    );
    assert.equal(
      vaultTokenAccountInfo.amount.toString(),
      DEPOSIT_AMOUNT.toString()
    );
  });

  it("withdraws tokens from the vault", async () => {
    await program.methods
      .withdraw(HALF_DEPOSIT_AMOUNT)
      .accountsPartial({
        authority: authority.publicKey,
        mint,
        vaultState: vaultStatePda,
        authorityTokenAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultState = await program.account.vaultState.fetch(vaultStatePda);
    assert.equal(
      vaultState.deposited.toString(),
      HALF_DEPOSIT_AMOUNT.toString()
    );

    const vaultTokenAccountInfo = await getAccount(
      provider.connection,
      vaultTokenAccountPda
    );
    assert.equal(
      vaultTokenAccountInfo.amount.toString(),
      HALF_DEPOSIT_AMOUNT.toString()
    );
  });

  it("rejects withdrawal exceeding deposited amount", async () => {
    const tooMuch = new anchor.BN(999_999_999);

    try {
      await program.methods
        .withdraw(tooMuch)
        .accountsPartial({
          authority: authority.publicKey,
          mint,
          vaultState: vaultStatePda,
          authorityTokenAccount,
          vaultTokenAccount: vaultTokenAccountPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      assert.fail("should have thrown");
    } catch (e) {
      assert.ok(
        e.message.includes("InsufficientFunds") || e.message.includes("6001"),
        `Expected InsufficientFunds error, got: ${e.message}`
      );
    }
  });

  it("closes an empty vault", async () => {
    await program.methods
      .withdraw(HALF_DEPOSIT_AMOUNT)
      .accountsPartial({
        authority: authority.publicKey,
        mint,
        vaultState: vaultStatePda,
        authorityTokenAccount,
        vaultTokenAccount: vaultTokenAccountPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    await program.methods
      .closeVault()
      .accountsPartial({
        authority: authority.publicKey,
        mint,
        vaultState: vaultStatePda,
        vaultTokenAccount: vaultTokenAccountPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const vaultStateInfo = await provider.connection.getAccountInfo(
      vaultStatePda
    );
    const vaultTokenAccountInfo = await provider.connection.getAccountInfo(
      vaultTokenAccountPda
    );

    assert.equal(vaultStateInfo, null);
    assert.equal(vaultTokenAccountInfo, null);
  });
});
