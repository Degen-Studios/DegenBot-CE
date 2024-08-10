This readme assumes you have familiarity with Git

## Step 1 - Install Rust

It has plenty of documentation, head on over to <a href="https://www.rust-lang.org/" target="_blank">https://www.rust-lang.org/</a> to learn how to install it and update it.

## Step 2 - Clone it

With SSH:

```shell
git clone git@github.com:Degen-Studios/DegenBot-CE.git
```

With HTTPS:

```shell
git clone https://github.com/Degen-Studios/DegenBot-CE.git
```

## Step 3 - Configure it

Get a bot token on Telegram from the BotFather
<a href="https://t.me/BotFather" target="_blank">@BotFather</a>

Edit `example.Secrets.toml` to include your new Bot Token and rename it to `Secrets.toml`

To change the "Welcome Message" go to `src/commands/start.rs` and edit the `response` variable value.

If you'd like to replace the "hands" from Degen POV you can find the existing ones in the `img` directory so you can be made aware of dimensions.

## Step 4 - Deploy

Deploy to Shuttle (if youo're unfamiliar it's like Vercel for NextJS and Heroku, except it's for Rust)

Here is the Shuttle Installation Directions for Shuttle
<a href="https://docs.shuttle.rs/getting-started/installation" target="_blank">Installation - Shuttle</a>

Because this is a bot there's no guarantee of web traffic, as such you should also follow the <a href="https://docs.shuttle.rs/getting-started/idle-projects" target="_blank">Idle Projects - Shuttle</a> documentation.

## Step 5 - Enjoy!
Note that there is information on how to run it locally with Shuttle as well.

### TODO

- Code Contribution Documentatoin
- Wiki Pages