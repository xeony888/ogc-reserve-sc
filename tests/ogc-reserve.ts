import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BN } from "bn.js";
import { PublicKey, Keypair } from "@solana/web3.js";
import { OgcReserve } from "../target/types/ogc_reserve";
import { assert } from "chai";
import { createAssociatedTokenAccount, createMint, getAccount, getAssociatedTokenAddressSync, mintTo } from "@solana/spl-token";

describe("ogc-reserve", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;
  const program = anchor.workspace.OgcReserve as Program<OgcReserve>;
  let mint: PublicKey;
  const mintToken = async () => {
    mint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      6,
    );
    const tokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      mint,
      wallet.publicKey,
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      mint,
      tokenAccount,
      wallet.payer,
      100000 * 10 ** 6
    )
    const second = new PublicKey("FUcoeKT9Nod5mWxDJJrbq4SycLAqNyxe5eMnmChbZ89p");
    const tokenAccount2 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      mint,
      second,
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      mint,
      tokenAccount2,
      wallet.payer,
      100000 * 10 ** 6
    )
  }
  it("initializes", async () => {
    await mintToken();
    // Add your test here.
    await program.methods.initialize().accounts({
      signer: wallet.publicKey,
      mint,
    }).rpc();
    await program.methods.createDataAccount().accounts({
      signer: wallet.publicKey,
      mint,
    }).rpc();
    const [globalAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    );
    const [epochAccount0Address] = PublicKey.findProgramAddressSync(
      [Buffer.from("epoch"), new BN(0).toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    const epochAccount0 = await program.account.epochAccount.fetch(epochAccount0Address);
    let globalAccount = await program.account.globalDataAccount.fetch(globalAccountAddress);
    assert(globalAccount.epoch.eq(new BN(0)), "Incorrect epoch");
    assert(epochAccount0.winner.eq(new BN(0)), "Incorrect winner");
    assert(!epochAccount0.fields.find(f => !f.eq(new BN(0))), "Incorrectly set fields");
    const tx2 = await program.methods.newEpoch(new BN(1)).accounts({
      signer: wallet.publicKey,
      prevEpochAccount: epochAccount0Address,
    }).rpc();
    const [epochAccount1Address] = PublicKey.findProgramAddressSync(
      [Buffer.from("epoch"), new BN(1).toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    const epochAccount1 = await program.account.epochAccount.fetch(epochAccount1Address);
    globalAccount = await program.account.globalDataAccount.fetch(globalAccountAddress);
    assert(epochAccount1.winner.eq(new BN(0)), "Incorrect winner");
    assert(globalAccount.epoch.eq(new BN(1)), "Incorrect epoch num");
  });
  it("funds", async () => {
    const signerTokenAccount = getAssociatedTokenAddressSync(mint, wallet.publicKey);
    await program.methods.depositOgg(new BN(100000 * 10 ** 6)).accounts({
      signer: wallet.publicKey,
      signerTokenAccount
    }).rpc();
    const [programHolderAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder")],
      program.programId
    );
    const programHolderAccount = await getAccount(provider.connection, programHolderAccountAddress);
    assert(new BN(programHolderAccount.amount.toString()).eq(new BN(100000 * 10 ** 6)), "Incorrect amount");
  });
  it("withdraws", async () => {
    const signerTokenAccount = getAssociatedTokenAddressSync(mint, wallet.publicKey);
    await program.methods.withdrawOgg(new BN(50000 * 10 ** 6)).accounts({
      signer: wallet.publicKey,
      signerTokenAccount
    }).rpc();
    const [programHolderAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder")],
      program.programId
    );
    const programHolderAccount = await getAccount(provider.connection, programHolderAccountAddress);
    const signerTokenAccountData = await getAccount(provider.connection, signerTokenAccount);
    assert(programHolderAccount.amount == signerTokenAccountData.amount, "Incorrect amount");
  })
  it("locks and unlocks", async () => {
      const signerTokenAccount = getAssociatedTokenAddressSync(mint, wallet.publicKey);
      await program.methods.createLockAccount(new BN(1)).accounts({
        signer: wallet.publicKey,
      }).rpc();
      await program.methods.lock(new BN(1), new BN(100)).accounts({
        signer: wallet.publicKey,
        signerTokenAccount,
      }).rpc();
      const [lockAccountAddress] = PublicKey.findProgramAddressSync(
        [Buffer.from("lock"), wallet.publicKey.toBuffer(), new BN(1).toArrayLike(Buffer, "le", 8)],
        program.programId
      )
      let lockAccount = await program.account.lockAccount.fetch(lockAccountAddress);
      assert(lockAccount.amount.eq(new BN(100)), "Incorrect amount");
      assert(lockAccount.unlockEpoch.eq(new BN(2)), "Incorrect unlock epoch");
      await program.methods.lock(new BN(1), new BN(100)).accounts({
        signer: wallet.publicKey,
        signerTokenAccount,
      }).rpc();
      lockAccount = await program.account.lockAccount.fetch(lockAccountAddress);
      assert(lockAccount.amount.eq(new BN(200)), "Incorrect amount");
      assert(lockAccount.unlockEpoch.eq(new BN(2)), "Incorrect unlock epoch");
      const [prevEpochAccount] = PublicKey.findProgramAddressSync(
        [Buffer.from("epoch"), new BN(1).toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      await program.methods.newEpoch(new BN(2)).accounts({
        signer: wallet.publicKey,
        prevEpochAccount
      }).rpc();
      await program.methods.unlock(new BN(1), new BN(100)).accounts({
        signer: wallet.publicKey,
        signerTokenAccount,
      }).rpc();
      lockAccount = await program.account.lockAccount.fetch(lockAccountAddress);
      assert(lockAccount.amount.eq(new BN(100)), "Incorrect amount");
      await program.methods.unlock(new BN(1), new BN(100)).accounts({
        signer: wallet.publicKey,
        signerTokenAccount,
      }).rpc();
      try {
        lockAccount = await program.account.lockAccount.fetch(lockAccountAddress);
        assert(false, "Lock account should have been deleted");
      } catch (e) {

      }
  })  
  it("votes and claims", async () => {
    await program.methods.createLockAccount(new BN(2)).accounts({
      signer: wallet.publicKey
    }).rpc();
    const signerTokenAccount = getAssociatedTokenAddressSync(mint, wallet.publicKey);
    await program.methods.lock(new BN(2), new BN(16000)).accounts({
      signer: wallet.publicKey,
      signerTokenAccount,
    }).rpc();
    const [dataAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("data"), wallet.publicKey.toBuffer()],
      program.programId,
    );
    const dataAccount = await program.account.userDataAccount.fetch(dataAccountAddress);
    assert(dataAccount.amount.eq(new BN(16000)));
    const data = [];
    for (let i = 0; i < 16; i++) {
      data.push(new BN(1000));
    }
    await program.methods.vote(new BN(2), data).accounts({
      signer: wallet.publicKey
    }).rpc();
    const [voteAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("vote"), wallet.publicKey.toBuffer(), new BN(2).toArrayLike(Buffer, "le", 8)],
      program.programId,
    )
    let voteAccount = await program.account.voteAccount.fetch(voteAccountAddress);
    assert(!voteAccount.fields.find(p => !p.eq(new BN(1000))), "Invalid field");
    const [prevEpochAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("epoch"), new BN(2).toArrayLike(Buffer, "le", 8)],
      program.programId,
    );
    await program.methods.newEpoch(new BN(3)).accounts({
      signer: wallet.publicKey,
      prevEpochAccount, 
    }).rpc();
    const [programHolderAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("holder")],
      program.programId
    );
    const [globalAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    )
    const globalAccount = await program.account.globalDataAccount.fetch(globalAccountAddress);
    const programHolderAccount = await getAccount(provider.connection, programHolderAccountAddress);
    const epochAccount2 = await program.account.epochAccount.fetch(prevEpochAccount);
    assert(epochAccount2.reward.eq(
      new BN(programHolderAccount.amount.toString()).mul(globalAccount.rewardPercent).div(new BN(100))), "Incorrect reward amount");
    const signerTokenAccountBefore = await getAccount(provider.connection, signerTokenAccount);
    await program.methods.claim(new BN(2)).accounts({
      signer: wallet.publicKey,
      signerTokenAccount,
    }).rpc();
    await new Promise(resolve => setTimeout(resolve, 1000));
    const signerTokenAccountAfter = await getAccount(provider.connection, signerTokenAccount);
    assert(signerTokenAccountAfter.amount > signerTokenAccountBefore.amount, "Did not get token");
    try {
      voteAccount = await program.account.voteAccount.fetch(voteAccountAddress);
      assert(false, "vote account not deleted");
    } catch (e) { }
  });
});
