import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Token3 } from "../target/types/token3";

import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getMint,
  getOrCreateAssociatedTokenAccount,
  createAssociatedTokenAccount,
  getAccount,
  createMint,
  mintTo,
  Account,
  transfer,
} from "@solana/spl-token";

import * as borsh from "@project-serum/borsh";

import { struct, u8 } from "@solana/buffer-layout";
import { u64 } from "@solana/buffer-layout-utils";

import fs from "fs";

const initialAmount = 100000;

let usdcPDA: PublicKey;
let usdcBump: Number;

let usdcMint: PublicKey;
let treasury: Account;

let tokenAuthority: Keypair;

let newAccount: Keypair;

describe("token3", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Token3 as Program<Token3>;

  const connection = anchor.getProvider().connection;
  const userWallet = anchor.workspace.Token3.provider.wallet;
  const randomPayer = async (lamports = LAMPORTS_PER_SOL) => {
    const wallet = Keypair.generate();
    const signature = await connection.requestAirdrop(
      wallet.publicKey,
      lamports
    );
    await connection.confirmTransaction(signature);
    return wallet;
  };

  before(async () => {
    tokenAuthority = Keypair.generate();
    const signature = await connection.requestAirdrop(
      tokenAuthority.publicKey,
      LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(signature);

    usdcMint = await createMint(
      connection, //connection to Solana
      await randomPayer(), //user randomPayer helper to create accounts for test
      tokenAuthority.publicKey, // mint authority
      null, // freeze authority (you can use `null` to disable it. when you disable it, you can't turn it on again)
      2, // decimals
      usdcMintKeypair
    );

    treasury = await getOrCreateAssociatedTokenAccount(
      connection, // connection to Solana
      await randomPayer(), // randomPayer for testing
      usdcMint, // Token Mint
      tokenAuthority.publicKey // user with Authority over this Token Account
    );

    // await mintTo(
    //   connection, // connection to Solana
    //   await randomPayer(), // randomPayer as payer for test
    //   usdcMint, // USDC Token Mint
    //   treasury.address, // User USDC Token Account (destination)
    //   tokenAuthority, // Mint Authority (required as signer)
    //   initialAmount
    // );

    // check tokens minted to Token Account
    const usdcAccount = await getAccount(connection, treasury.address);
    console.log("USDC Mint:", usdcMint.toString());
    console.log("setup USDC Token Account:", Number(usdcAccount.amount));
  });

  it("New Token", async () => {
    newAccount = Keypair.generate();

    const [tokenPDA, tokenBump] = await PublicKey.findProgramAddress(
      [Buffer.from("MINT"), newAccount.publicKey.toBuffer()],
      program.programId
    );

    const [earnedPDA, earnedBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("EARNED"),
        newAccount.publicKey.toBuffer(),
        usdcMintAddress.toBuffer(),
      ],
      program.programId
    );

    const [reservePDA, reserveBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from("RESERVE"),
        newAccount.publicKey.toBuffer(),
        usdcMintAddress.toBuffer(),
      ],
      program.programId
    );

    try {
      await program.rpc.newToken(
        "token",
        new anchor.BN(1),
        new anchor.BN(1),
        new anchor.BN(1),
        new anchor.BN(1),
        {
          accounts: {
            tokenData: newAccount.publicKey,
            tokenMint: tokenPDA,
            earnedUsdcAccount: earnedPDA,
            reserveUsdcAccount: reservePDA,
            mint: usdcMintAddress,
            user: userWallet.publicKey,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [newAccount],
        }
      );
    } catch (error) {
      console.log(error);
    }

    const token = await program.account.tokenData.fetch(newAccount.publicKey);
    console.log(token.mint.toString());
    console.log(tokenPDA.toString());
    console.log(token.earned.toString());
    console.log(earnedPDA.toString());
    console.log(token.reserve.toString());
    console.log(reservePDA.toString());
    console.log(token.user.toString());
    // console.log(token);
  });

  // it("Mint Token", async () => {
  //   const [tokenPDA, tokenBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("MINT"), newAccount.publicKey.toBuffer()],
  //     program.programId
  //   );

  //   const TokenAccount = await getOrCreateAssociatedTokenAccount(
  //     connection,
  //     await randomPayer(),
  //     tokenPDA,
  //     provider.wallet.publicKey
  //   );

  //   const usdcTokenAccount = await getOrCreateAssociatedTokenAccount(
  //     connection,
  //     await randomPayer(),
  //     usdcMint,
  //     provider.wallet.publicKey
  //   );

  //   const token = await program.account.tokenData.fetch(newAccount.publicKey);

  //   await mintTo(
  //     connection, // connection to Solana
  //     await randomPayer(), // randomPayer as payer for test
  //     usdcMint, // USDC Token Mint
  //     usdcTokenAccount.address, // User USDC Token Account (destination)
  //     tokenAuthority, // Mint Authority (required as signer)
  //     initialAmount
  //   );

  //   try {
  //     await program.rpc.mintToken(new anchor.BN(initialAmount), {
  //       accounts: {
  //         tokenData: newAccount.publicKey,
  //         tokenMint: tokenPDA,
  //         reserveUsdcAccount: token.reserve,
  //         treasuryAccount: treasury.address,
  //         userToken: TokenAccount.address,
  //         userUsdcToken: usdcTokenAccount.address,
  //         user: provider.wallet.publicKey,
  //         mint: usdcMint,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //     });
  //   } catch (error) {
  //     console.log(error);
  //   }

  //   const balance1 = await getMint(connection, tokenPDA);

  //   const balance2 = (await connection.getTokenAccountBalance(token.reserve))
  //     .value.amount;

  //   const balance3 = (await connection.getTokenAccountBalance(treasury.address))
  //     .value.amount;

  //   const balance4 = (
  //     await connection.getTokenAccountBalance(TokenAccount.address)
  //   ).value.amount;

  //   const balance5 = (
  //     await connection.getTokenAccountBalance(usdcTokenAccount.address)
  //   ).value.amount;

  //   // console.log("Token Supply Balance:", balance2);
  //   console.log("Token Mint Supply:", Number(balance1.supply));
  //   console.log("reserve Balance:", balance2);
  //   console.log("treasury Balance:", balance3);
  //   console.log("userToken Balance:", balance4);
  //   console.log("userUSDC Balance:", balance5);
  // });

  // it("Redeem", async () => {
  //   const Wallet = Keypair.generate();
  //   const AirdropSignature = await connection.requestAirdrop(
  //     Wallet.publicKey,
  //     LAMPORTS_PER_SOL
  //   );

  //   await connection.confirmTransaction(AirdropSignature);

  //   const [tokenPDA, tokenBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("MINT"), newAccount.publicKey.toBuffer()],
  //     program.programId
  //   );

  //   // Get the token account of the fromWallet address, and if it does not exist, create it
  //   const TokenAccount = await getOrCreateAssociatedTokenAccount(
  //     connection,
  //     await randomPayer(),
  //     tokenPDA,
  //     Wallet.publicKey
  //   );

  //   const usdcTokenAccount = await getOrCreateAssociatedTokenAccount(
  //     connection,
  //     await randomPayer(),
  //     usdcMint,
  //     Wallet.publicKey
  //   );

  //   await mintTo(
  //     connection, // connection to Solana
  //     await randomPayer(), // randomPayer as payer for test
  //     usdcMint, // USDC Token Mint
  //     usdcTokenAccount.address, // User USDC Token Account (destination)
  //     tokenAuthority, // Mint Authority (required as signer)
  //     initialAmount
  //   );

  //   const token = await program.account.tokenData.fetch(newAccount.publicKey);

  //   try {
  //     await program.rpc.mintToken(new anchor.BN(initialAmount), {
  //       accounts: {
  //         tokenData: newAccount.publicKey,
  //         tokenMint: token.mint,
  //         reserveUsdcAccount: token.reserve,
  //         treasuryAccount: treasury.address,
  //         userToken: TokenAccount.address,
  //         userUsdcToken: usdcTokenAccount.address,
  //         user: Wallet.publicKey,
  //         mint: usdcMint,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [Wallet],
  //     });
  //   } catch (error) {
  //     console.log(error);
  //   }

  //   try {
  //     await program.rpc.redeem(new anchor.BN(initialAmount / 10), {
  //       accounts: {
  //         tokenData: newAccount.publicKey,
  //         tokenMint: token.mint,
  //         userToken: TokenAccount.address,
  //         user: Wallet.publicKey,
  //         reserveUsdcAccount: token.reserve,
  //         earnedUsdcAccount: token.earned,
  //         treasuryAccount: treasury.address,
  //         mint: usdcMint,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [Wallet],
  //     });
  //   } catch (error) {
  //     console.log(error);
  //   }

  //   const balance1 = await getMint(connection, tokenPDA);

  //   const balance2 = (await connection.getTokenAccountBalance(token.reserve))
  //     .value.amount;

  //   const balance3 = (await connection.getTokenAccountBalance(treasury.address))
  //     .value.amount;

  //   const balance4 = (
  //     await connection.getTokenAccountBalance(TokenAccount.address)
  //   ).value.amount;

  //   const balance5 = (
  //     await connection.getTokenAccountBalance(usdcTokenAccount.address)
  //   ).value.amount;

  //   const balance6 = (await connection.getTokenAccountBalance(token.earned))
  //     .value.amount;

  //   // console.log("Token Supply Balance:", balance2);
  //   console.log("Token Mint Supply:", Number(balance1.supply));
  //   console.log("reserve Balance:", balance2);
  //   console.log("treasury Balance:", balance3);
  //   console.log("earned Balance:", balance6);
  //   console.log("userToken Balance:", balance4);
  //   console.log("userUSDC Balance:", balance5);
  // });

  it("Partial Redeem", async () => {
    const Wallet = Keypair.generate();
    const AirdropSignature = await connection.requestAirdrop(
      Wallet.publicKey,
      LAMPORTS_PER_SOL
    );

    await connection.confirmTransaction(AirdropSignature);

    const [tokenPDA, tokenBump] = await PublicKey.findProgramAddress(
      [Buffer.from("MINT"), newAccount.publicKey.toBuffer()],
      program.programId
    );

    // Get the token account of the fromWallet address, and if it does not exist, create it
    const TokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      await randomPayer(),
      tokenPDA,
      Wallet.publicKey
    );

    const usdcTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      await randomPayer(),
      usdcMint,
      Wallet.publicKey
    );

    await mintTo(
      connection, // connection to Solana
      await randomPayer(), // randomPayer as payer for test
      usdcMint, // USDC Token Mint
      usdcTokenAccount.address, // User USDC Token Account (destination)
      tokenAuthority, // Mint Authority (required as signer)
      initialAmount
    );

    const token = await program.account.tokenData.fetch(newAccount.publicKey);

    try {
      await program.rpc.mintToken(new anchor.BN(initialAmount), {
        accounts: {
          tokenData: newAccount.publicKey,
          tokenMint: token.mint,
          reserveUsdcAccount: token.reserve,
          treasuryAccount: treasury.address,
          userToken: TokenAccount.address,
          userUsdcToken: usdcTokenAccount.address,
          user: Wallet.publicKey,
          mint: usdcMint,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [Wallet],
      });
    } catch (error) {
      console.log(error);
    }

    try {
      await program.rpc.partialRedeem(new anchor.BN(100), new anchor.BN(100), {
        accounts: {
          tokenData: newAccount.publicKey,
          tokenMint: token.mint,
          userToken: TokenAccount.address,
          userUsdcToken: usdcTokenAccount.address,
          user: Wallet.publicKey,
          reserveUsdcAccount: token.reserve,
          earnedUsdcAccount: token.earned,
          treasuryAccount: treasury.address,
          mint: usdcMint,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [Wallet],
      });
    } catch (error) {
      console.log(error);
    }

    const balance1 = await getMint(connection, tokenPDA);

    const balance2 = (await connection.getTokenAccountBalance(token.reserve))
      .value.amount;

    const balance3 = (await connection.getTokenAccountBalance(treasury.address))
      .value.amount;

    const balance4 = (
      await connection.getTokenAccountBalance(TokenAccount.address)
    ).value.amount;

    const balance5 = (
      await connection.getTokenAccountBalance(usdcTokenAccount.address)
    ).value.amount;

    const balance6 = (await connection.getTokenAccountBalance(token.earned))
      .value.amount;

    // console.log("Token Supply Balance:", balance2);
    console.log("Token Mint Supply:", Number(balance1.supply));
    console.log("reserve Balance:", balance2);
    console.log("treasury Balance:", balance3);
    console.log("earned Balance:", balance6);
    console.log("userToken Balance:", balance4);
    console.log("userUSDC Balance:", balance5);
  });

  it("Withdraw", async () => {
    const usdcTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      await randomPayer(),
      usdcMint,
      provider.wallet.publicKey
    );

    const token = await program.account.tokenData.fetch(newAccount.publicKey);

    const balance1 = (await connection.getTokenAccountBalance(token.earned))
      .value.amount;

    const balance6 = (
      await connection.getTokenAccountBalance(usdcTokenAccount.address)
    ).value.amount;

    console.log("before earned Balance:", balance1);
    console.log("before userUSDC Balance:", balance6);

    try {
      await program.rpc.withdraw({
        accounts: {
          tokenData: newAccount.publicKey,
          earnedUsdcAccount: token.earned,
          withdrawUsdcAccount: usdcTokenAccount.address,
          mint: usdcMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          authority: provider.wallet.publicKey,
        },
      });
    } catch (error) {
      console.log(error);
    }

    const balance2 = (await connection.getTokenAccountBalance(token.earned))
      .value.amount;

    const balance5 = (
      await connection.getTokenAccountBalance(usdcTokenAccount.address)
    ).value.amount;

    // console.log("Token Supply Balance:", balance2);
    console.log("after earned Balance:", balance2);
    console.log("after userUSDC Balance:", balance5);
  });

  it("Update Token Data", async () => {
    const before = await program.account.tokenData.fetch(newAccount.publicKey);
    console.log(before.name);
    console.log(before.discount.toNumber());
    console.log(before.reward.toNumber());

    try {
      await program.rpc.updateTokenData(
        "update",
        new anchor.BN(2),
        new anchor.BN(2),
        {
          accounts: {
            tokenData: newAccount.publicKey,
            user: userWallet.publicKey,
          },
        }
      );
    } catch (error) {
      console.log(error);
    }

    const after = await program.account.tokenData.fetch(newAccount.publicKey);
    console.log(after.name);
    console.log(after.discount.toNumber());
    console.log(after.reward.toNumber());
  });
});

// @ts-ignore
// solana-keygen new --outfile .keys/usdc_mint.json
const usdcData = JSON.parse(fs.readFileSync(".keys/usdc_mint.json"));
const usdcMintKeypair = Keypair.fromSecretKey(new Uint8Array(usdcData));
const usdcMintAddress = usdcMintKeypair.publicKey;
console.log("USDC Mint:", usdcMintAddress.toString());
