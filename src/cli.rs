use clap::{Arg, Command};

pub fn cli() -> Command {
// Recognize argument and call appropriate function
let matches = Command::new("ash")
        .about("Any Snapshot Hierarchical OS")
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Auto upgrade
        .subcommand(Command::new("auto-upgrade")
                    .aliases(["autoup", "au"])
                    .about("Update a snapshot quietly")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),) // REVIEW any given snapshot or get_current_snapshot() ?
        // Base update
        .subcommand(Command::new("base-update")
                    .alias("bu")
                    .about("Update the base snapshot"))
        // Boot update command
        .subcommand(Command::new("boot")
                    .alias("boot-update")
                    .about("update boot of a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Branch
        .subcommand(Command::new("branch")
                    .alias("add-branch")
                    .about("Create a new branch from snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("desc")
                         .long("desc")
                         .alias("description")
                         .short('d')
                         .num_args(1..)
                         .required(false)
                         .help("description for the snapshot"),),)
        // Check update
        .subcommand(Command::new("check")
                    .about("Check update"))
        // Chroot
        .subcommand(Command::new("chroot")
                    .aliases(["chr", "ch"])
                    .about("Open a root shell inside a snapshot")
                    .arg(Arg::new("SNAPSHOT")
                         .long("snapshot")
                         .alias("snap")
                         .short('s')
                         .value_parser(clap::value_parser!(i32))
                         .required(false)
                         .help("Snapshot number"),)
                    .arg(Arg::new("COMMAND")
                         .long("command")
                         .alias("cmd")
                         .short('c')
                         .num_args(1..)
                         .required(false)
                         .help("Run command in snapshot"),),)
        // Clone
        .subcommand(Command::new("clone")
                    .alias("cl")
                    .about("Create a copy of a snapshot (as top-level tree node)")
                    .arg(Arg::new("SNAPSHOT")
                         .long("snapshot")
                         .alias("snap")
                         .short('s')
                         .value_parser(clap::value_parser!(i32))
                         .required(false)
                         .help("snapshot number"),)
                    .arg(Arg::new("DESCRIPTION")
                         .long("description")
                         .alias("desc")
                         .short('d')
                         .num_args(1..)
                         .required(false)
                         .help("description for the snapshot"),),)
        // Clone a branch
        .subcommand(Command::new("clone-branch")
                    .alias("cb")
                    .about("Copy snapshot under same parent branch (clone as a branch)")
                    .arg(Arg::new("SNAPSHOT")
                         .long("snapshot")
                         .alias("snap")
                         .short('s')
                         .value_parser(clap::value_parser!(i32))
                         .required(false)
                         .help("snapshot number"),),)
        // Clone recursively
        .subcommand(Command::new("clone-tree")
                    .alias("ct")
                    .about("clone a whole tree recursively")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Clone under a branch
        .subcommand(Command::new("clone-under")
                    .aliases(["cu", "ubranch"])
                    .about("Copy snapshot under specified parent (clone under a branch)")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("branch")
                         .value_parser(clap::value_parser!(i32))
                         .help("branch number"),),)
        // Current snapshot
        .subcommand(Command::new("current")
                    .alias("c")
                    .external_subcommand_value_parser(clap::value_parser!(i32))
                    .about("Show current snapshot number"))
        // Delete
        .subcommand(Command::new("del")
                    .aliases(["delete", "rem", "remove"])
                    .about("Remove snapshot(s)/tree(s) and any branches recursively")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshots")
                         .num_args(1..)
                         .help("snapshot number"),)
                    .arg(Arg::new("--quiet")
                         .alias("-q")
                         //.action='store_true'
                         .required(false)
                         .help("Force delete snapshot(s)"),),)
        // Deploy
        .subcommand(Command::new("deploy")
                    .aliases(["dep", "d"])
                    .about("Deploy a snapshot for next boot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Description
        .subcommand(Command::new("desc")
                    .about("set a description for a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("desc")
                         .num_args(1..)
                         .help("description to be added"),),)
        // Diff two snapshots
        .subcommand(Command::new("diff")
                    .alias("dif")
                    .about("Show package diff between snapshots")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snap1")
                         .value_parser(clap::value_parser!(i32))
                         .help("Source snapshot"),)
                    .arg(Arg::new("snap2")
                         .value_parser(clap::value_parser!(i32))
                         .help("Target snapshot"),),)
        // Edit Ash configuration
        .subcommand(Command::new("edit")
                    .alias("edit-conf")
                    .about("Edit configuration for a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // etc update
        .subcommand(Command::new("etc-update")
                    .alias("etc")
                    .about("update /etc"))
        // Fix db command ### MAYBE ash_unlock was needed?
        .subcommand(Command::new("fixdb")
                    .alias("fix")
                    .about("fix package database of a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Hollow a node
        .subcommand(Command::new("hollow")
                    .about("Make a snapshot hollow (vulnerable)")
                    .arg_required_else_help(true)
                    .arg(Arg::new("s")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Immutability disable
        .subcommand(Command::new("immdis")
                    .aliases(["disimm", "immdisable", "disableimm"])
                    .about("Disable immutability of a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Immutability enable
        .subcommand(Command::new("immen")
                    .aliases(["enimm", "immenable", "enableimm"])
                    .about("Enable immutability of a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Install command
        .subcommand(Command::new("install")
                    .alias("in")
                    .about("install package(s) inside a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("--pkg")
                         .aliases([ "--package", "-p" ])
                         .num_args(1..)
                         .required_unless_present("--profile")
                         .conflicts_with("--profile")
                         .help("install package"),)
                    .arg(Arg::new("--profile")
                         .alias("-P" )
                         .value_parser(clap::value_parser!(String))
                         .required_unless_present("--pkg")
                         .conflicts_with("--profile")
                         .help("install profile"),),)
        // Live chroot
        .subcommand(Command::new("live-chroot")
                    .aliases(["lchroot", "lc"])
                    .about("Open a shell inside currently booted snapshot with read-write access. Changes are discarded on new deployment."))
        // New
        .subcommand(Command::new("new")
                    .alias("new-tree")
                    .about("Create a new base snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("--desc")
                         .aliases(["--description", "-d"])
                         .num_args(1..)
                         .required(false)
                         .help("Description for the snapshot"),),)
        // Refresh
        .subcommand(Command::new("refresh")
                    .alias("ref")
                    .about("Refresh package manager db of a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Rollback
        .subcommand(Command::new("rollback")
                    .about("Revert the deployment to the last booted snapshot"))
        // Run a command
        .subcommand(Command::new("run")
                    .about("Run command(s) inside another snapshot (chrooted)")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("cmd")
                         .num_args(1..)
                         .help("command"),),)
        // Subvolumes list
        .subcommand(Command::new("subs")
                    .aliases(["sub", "subvol", "subvols", "subvolumes"])
                    .about("List subvolumes of active snapshot (currently booted)"))
        // Tree-Sync
        .subcommand(Command::new("sync")
                    .aliases(["tree-sync", "tsync"])
                    .about("Sync packages and configuration changes recursively (requires an internet connection)")
                    .arg_required_else_help(true)
                    .arg(Arg::new("treename")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("-f")
                         .aliases(["--force", "--force-offline"])
                         //.action='store_true'
                         .required(false)
                         .help("Snapshots would not get updated (potentially riskier)"),)
                    .arg(Arg::new("--not-live")
                         .alias("-nl")
                         //.action='store_true'
                         .required(false)
                         .help("Disable live sync"),),)
        // tmp (clear tmp)
        .subcommand(Command::new("tmp")
                    .alias("tmpclear")
                    .about("Show ash tree"))
        // Tree
        .subcommand(Command::new("tree")
                    .alias("t")
                    .about("Show ash tree"))
        // Tree-remove
        .subcommand(Command::new("tremove")
                    .alias("tree-rmpkg")
                    .about("Uninstall package(s) or profile(s) from a tree recursively")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("--pkg")
                         .aliases([ "--package", "-p" ])
                         .num_args(1..)
                         .required_unless_present("--profile")
                         .conflicts_with("--profile")
                         .help("package(s) to be uninstalled"),)
                    .arg(Arg::new("--profile")
                         .alias("-P" )
                         .value_parser(clap::value_parser!(String))
                         .required_unless_present("--pkg")
                         .conflicts_with("--profile")
                         .help("profile(s) to be uninstalled"),),)
        // Tree-run
        .subcommand(Command::new("trun")
                    .alias("tree-run")
                    .about("Execute command(s) inside another snapshot and all snapshots below it")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("--cmd")
                         .aliases(["--command", "-c"])
                         .num_args(1..)
                         .required(false)
                         .help("command(s) to run"),),)
        // Tree-upgrade
        .subcommand(Command::new("tupgrade")
                    .aliases(["tree-upgrade", "tup"])
                    .about("Update all packages in a snapshot recursively")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Uninstall package(s) from a snapshot
        .subcommand(Command::new("uninstall")
                    .aliases(["unin", "uninst", "unins", "un"])
                    .about("Uninstall package(s) from a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),)
                    .arg(Arg::new("--pkg")
                         .aliases([ "--package", "-p" ])
                         .num_args(1..)
                         .required_unless_present("--profile")
                         .conflicts_with("--profile")
                         .help("package(s) to be uninstalled"),)
                    .arg(Arg::new("--profile")
                         .alias("-P" )
                         .value_parser(clap::value_parser!(String))
                         .required_unless_present("--pkg")
                         .conflicts_with("--profile")
                         .help("profile(s) to be uninstalled"),),)
        // Unlock a snapshot
        .subcommand(Command::new("unlock")
                    .alias("ul")
                    .about("Unlock a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snap")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Upgrade a snapshot
        .subcommand(Command::new("upgrade")
                    .alias("up")
                    .about("Update all packages in a snapshot")
                    .arg_required_else_help(true)
                    .arg(Arg::new("snapshot")
                         .value_parser(clap::value_parser!(i32))
                         .help("snapshot number"),),)
        // Ash version
        .subcommand(Command::new("version")
                    .alias("v")
                    .about("Print ash version"))
        // Which deployment is active
        .subcommand(Command::new("whichtmp")
                    .aliases(["whichdep", "which"])
                    .about("Show which deployment snapshot is in use"));
    return matches;
}