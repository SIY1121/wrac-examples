use std::error::Error;

use clap::Parser;

mod cli;
mod commands;
mod context;
mod metadata;
mod profile;
mod targets;
mod util;

use cli::{Cli, Commands};
use commands::{build, clean, install, launch, uninstall, validate};
use context::{Context, available_plugins};
use profile::BuildProfile;

pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => {
            // Keep the command implementation single-plugin oriented, just like the template.
            // The examples repository adds multiplicity at this dispatch layer so build/install
            // logic stays close to upstream and each plugin still gets an isolated Context.
            for plugin in selected_plugins(args.plugin.as_deref(), args.all)? {
                let ctx = Context::new(&plugin)?;
                build(&ctx, args_for_build(&args))?;
            }
        }
        Commands::Install(args) => {
            for plugin in selected_plugins(args.plugin.as_deref(), args.all)? {
                let ctx = Context::new(&plugin)?;
                install(
                    &ctx,
                    BuildProfile::from_release(args.release),
                    args.scope,
                    &args.target,
                )?;
            }
        }
        Commands::Uninstall(args) => {
            for plugin in selected_plugins(args.plugin.as_deref(), args.all)? {
                let ctx = Context::new(&plugin)?;
                uninstall(&ctx, args.scope, &args.target, args.dry_run)?;
            }
        }
        Commands::Validate(args) => {
            for plugin in selected_plugins(args.plugin.as_deref(), args.all)? {
                let ctx = Context::new(&plugin)?;
                validate(&ctx, BuildProfile::from_release(args.release), &args.target)?;
            }
        }
        Commands::Launch(args) => {
            let ctx = Context::new(&args.plugin)?;
            launch(&ctx, BuildProfile::from_release(args.release))?;
        }
        Commands::Clean(args) => {
            for plugin in selected_plugins(args.plugin.as_deref(), args.all)? {
                let ctx = Context::new(&plugin)?;
                clean(&ctx)?;
            }
        }
    }

    Ok(())
}

fn selected_plugins(plugin: Option<&str>, all: bool) -> Result<Vec<String>> {
    if all {
        if plugin.is_some() {
            return Err("--plugin and --all cannot be used together".into());
        }
        return available_plugins();
    }
    if let Some(plugin) = plugin {
        return Ok(vec![plugin.to_string()]);
    }
    Err("--plugin <PLUGIN> or --all is required".into())
}

fn args_for_build(args: &cli::BuildArgs) -> cli::BuildArgs {
    // Build is the only command where the command object is passed onward. Strip the
    // repository-level selection flags before handing it to template-derived build code.
    cli::BuildArgs {
        plugin: None,
        all: false,
        release: args.release,
        clean: args.clean,
        target: args.target.clone(),
    }
}
