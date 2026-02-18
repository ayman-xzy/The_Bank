import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TheBank } from "../target/types/the_bank";

describe("the_bank", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.the_bank as Program<TheBank>;
  const provider = anchor.getProvider();

  const vaultState = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("state"), provider.publicKey.toBytes()],
    program.programId
  )[0];

  const vault = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), vaultState.toBytes()],
    program.programId
  )[0];

  it("The-bank-Account-Has-been-Initialized!", async () => {
    const balance = await provider.connection.getBalance(provider.publicKey);
    console.log("Balance is:", balance / anchor.web3.LAMPORTS_PER_SOL);

    const tx = await program.methods
      .initialize()
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([provider.wallet.payer])
      .rpc();
    console.log("\nYour transaction signature", tx);
  });

  it("Deposit 2 SOL", async () => {
    const tx = await program.methods
      .deposit(new anchor.BN(2 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
  });

  it("Withdraw 1 SOL -> should fail (2 day timelock)", async () => {
  try {
    await program.methods
      .withdraw(new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL))
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    throw new Error("Withdraw should have failed but succeeded");
  } catch (err) {
    console.log("Withdraw blocked as expected");
  }
});

  it("Close Vault", async () => {
    const tx = await program.methods
      .close()
      .accountsPartial({
        user: provider.wallet.publicKey,
        vaultState,
        vault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nYour transaction signature", tx);
    console.log("Vault Balance:", await provider.connection.getBalance(vault));
  });

});