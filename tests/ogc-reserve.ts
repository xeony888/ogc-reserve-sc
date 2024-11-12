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
  let ogcMint: PublicKey;
  let oggMint: PublicKey;
  const mintToken = async () => {
    ogcMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      6,
    );
    const tokenAccount = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      ogcMint,
      wallet.publicKey,
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      ogcMint,
      tokenAccount,
      wallet.payer,
      100000 * 10 ** 6
    )
    const second = new PublicKey("58V6myLoy5EVJA3U2wPdRDMUXpkwg8Vfw5b6fHqi2mEj");
    const tokenAccount2 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      ogcMint,
      second,
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      ogcMint,
      tokenAccount2,
      wallet.payer,
      100000 * 10 ** 6
    )
    oggMint = await createMint(
      provider.connection,
      wallet.payer,
      wallet.publicKey,
      wallet.publicKey,
      6,
    );
    const tokenAccount3 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      oggMint,
      wallet.publicKey
    )
    const tokenAccount4 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      oggMint,
      second
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      oggMint,
      tokenAccount3,
      wallet.payer,
      100000 * 10 ** 6
    )
    await mintTo(
      provider.connection,
      wallet.payer,
      oggMint,
      tokenAccount4,
      wallet.payer,
      100000 * 10 ** 6
    )
  }
  const mintTokensTo = async () => {
    const token1 = new PublicKey("HHM13rXbmED6iKzJ2RWxQ4ALjqBy4inEuytxqKA8bhCD");
    const token2 = new PublicKey("GqUTe9ovCizU4DcHZ3QzUNa3UB1n2h8a17nVUaymcsxa");
    const address = new PublicKey("58V6myLoy5EVJA3U2wPdRDMUXpkwg8Vfw5b6fHqi2mEj");
    const tokenAccount1 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      token1,
      address
    );
    const tokenAccount2 = await createAssociatedTokenAccount(
      provider.connection,
      wallet.payer,
      token2,
      address
    );
    await mintTo(
      provider.connection,
      wallet.payer,
      token1,
      tokenAccount1,
      wallet.payer,
      100000 * 10 ** 6
    )
    await mintTo(
      provider.connection,
      wallet.payer,
      token2,
      tokenAccount2,
      wallet.payer,
      100000 * 10 ** 6
    )
  }
  it("initializes", async () => {
    await mintToken();
    console.log({ ogcMint: ogcMint.toString(), oggMint: oggMint.toString() })
    await program.methods.initializeFirstEpochAccount().accounts({
      signer: wallet.publicKey
    }).rpc();
      await program.methods.initialize().accounts({
        signer: wallet.publicKey,
        ogcMint,
        oggMint
      }).rpc();
    console.log("initialized");
    await program.methods.createDataAccount().accounts({
      signer: wallet.publicKey,
      mint: oggMint,
    }).rpc();
    await program.methods.createStatsAccount().accounts({
      signer: wallet.publicKey
    }).rpc();
    console.log("data account created")
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
    const signerTokenAccount = getAssociatedTokenAddressSync(ogcMint, wallet.publicKey);
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
    const signerTokenAccount = getAssociatedTokenAddressSync(ogcMint, wallet.publicKey);
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
      const signerTokenAccount = getAssociatedTokenAddressSync(oggMint, wallet.publicKey);
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
    const signerTokenAccount = getAssociatedTokenAddressSync(oggMint, wallet.publicKey);
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
    for (let i = 0; i < 4; i++) {
      data.push(new BN(500));
    }
    await program.methods.createVoteAccount(new BN(2)).accounts({
      signer: wallet.publicKey
    }).rpc();
    await program.methods.vote(new BN(2), data).accounts({
      signer: wallet.publicKey
    }).rpc();
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
    const [userStatsAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("stats"), wallet.publicKey.toBuffer()],
      program.programId
    );
    const globalAccount = await program.account.globalDataAccount.fetch(globalAccountAddress);
    const epochAccount2 = await program.account.epochAccount.fetch(prevEpochAccount);
    assert(epochAccount2.reward.eq(globalAccount.rewardAmount), "Incorrect reward amount");
    assert(epochAccount2.voters.eq(new BN(2)), "Invalid amount of voters"); // new check
    const signerTokenAccountAddress = getAssociatedTokenAddressSync(ogcMint, wallet.publicKey);
    const signerTokenAccountBefore = await getAccount(provider.connection, signerTokenAccountAddress);
    const userStatsAccountBefore = await program.account.userStatsAccount.fetch(userStatsAccountAddress)
    await program.methods.claim(new BN(2)).accounts({
      signer: wallet.publicKey,
      signerTokenAccount: signerTokenAccountAddress,
    }).rpc();
    const userStatsAccountAfter = await program.account.userStatsAccount.fetch(userStatsAccountAddress);
    assert(userStatsAccountBefore.amountClaimed.lte(userStatsAccountAfter.amountClaimed));
    await new Promise(resolve => setTimeout(resolve, 1000));
    const signerTokenAccountAfter = await getAccount(provider.connection, signerTokenAccountAddress);
    assert(signerTokenAccountAfter.amount > signerTokenAccountBefore.amount, "Did not get token");
    try {
      voteAccount = await program.account.voteAccount.fetch(voteAccountAddress);
      assert(false, "vote account not deleted");
    } catch (e) { }
  });
  it("modifies global data", async () => {
    await program.methods.modifyGlobalData(new BN(100), new BN(100), new BN(100)).accounts({
      signer: wallet.publicKey
    }).rpc();
    const [globalDataAccountAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("global")],
      program.programId
    );
    const globalDataAccount = await program.account.globalDataAccount.fetch(globalDataAccountAddress);
    assert(globalDataAccount.epochLength.eq(globalDataAccount.rewardAmount) && globalDataAccount.epochLength.eq(globalDataAccount.epochLockTime), "Incorrect parameter setting")
  })
  it("withdraws sol", async () => {
    const [programAuthorityAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("auth")],
      program.programId
    );
    const balanceBefore = await provider.connection.getBalance(programAuthorityAddress);
    await program.methods.withdrawSol().accounts({
      signer: wallet.publicKey,
    }).rpc();
    await new Promise((resolve) => setTimeout(resolve, 1000));
    const balanceAfter = await provider.connection.getBalance(programAuthorityAddress);
    assert(balanceAfter < balanceBefore, "Balance did not decrease");
  })
});
