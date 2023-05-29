mod detect_distro;
mod distros;
mod tree;

use crate::detect_distro as detect;
use crate::distros::*;
use tree::*;
use std::fs::{File, OpenOptions, read_dir, read_to_string};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// Ash version
//pub fn ash_version() {
    //let ash_bin_path = Path::new("/usr/sbin/ash");
    //let metadata = metadata(ash_bin_path).unwrap();
    //let time = metadata.modified().unwrap();
    //let duration = time.duration_since(UNIX_EPOCH).unwrap();
    //let utc = time::OffsetDateTime::UNIX_EPOCH +
        //time::Duration::try_from(duration).unwrap();
    //let local = utc.to_offset(time::UtcOffset::local_offset_at(utc).unwrap());
    //local.format_into(
        //&mut std::io::stdout().lock(),
        //time::macros::format_description!(
            //"[day]-[month repr:short]-[year] [hour]:[minute]:[second]\n"
        //),
    //).unwrap();
//}

// Ash chroot mounts
pub fn ash_chroot_mounts(i: &str) {
    let chr = "";
    Command::new("mount").arg("--bind")
                         .arg("--make-slave")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}", chr,i))
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}", chr,i)).status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/dev"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/dev", chr,i)).status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/etc"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/etc", chr,i)).status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/home"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/home", chr,i)).status().unwrap();
    Command::new("mount").args(["--types", "proc", "/proc"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/proc", chr,i)).status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/run"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/run", chr,i)).status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/sys"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/sys", chr,i)).status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/tmp"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/tmp", chr,i)).status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/var"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}{}/var", chr,i)).status().unwrap();
    if is_efi() {
        Command::new("mount").args(["--rbind", "--make-rslave", "/sys/firmware/efi/efivars"])
                             .arg(format!("/.snapshots/rootfs/snapshot-{}{}/sys/firmware/efi/efivars", chr,i)).status().unwrap();
        Command::new("cp").args(["--dereference", "/etc/resolv.conf"])
                          .arg(format!("/.snapshots/rootfs/snapshot-{}{}/etc/", chr,i)).status().unwrap();
        }
}

// Ash version
pub fn ash_version() {
    let version = String::from_utf8_lossy(&Command::new("date").arg("-r")
                                      .arg("/usr/sbin/ash")
                                      .arg("+%Y%m%d-%H%M%S")
                                      .output().unwrap().stdout).to_string();
    println!("{}", version);
}

// Check if snapshot is mutable
pub fn check_mutability(snapshot: &str) -> bool {
    Path::new(&format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", snapshot))
        .try_exists().unwrap()
}

// Check if last update was successful
pub fn check_update() {
    let upstate = File::open("/.snapshots/ash/upstate").unwrap();
    let buf_read = BufReader::new(upstate);
    let mut read = buf_read.lines();
    let line = read.next().unwrap().unwrap();
    let data = read.next().unwrap().unwrap();
    if line.contains("1") {
        eprintln!("Last update on {} failed.", data);
    }
    if line.contains("0") {
        print!("Last update on {} completed successfully.", data)
    }
}

// Clean chroot mount directories for a snapshot
pub fn chr_delete(snapshot: &str) {
    if Path::new(&format!("/.snapshots/rootfs/snapshot-chr{}", snapshot)).try_exists().unwrap() {
        Command::new("btrfs").args(["sub", "del"])
                             .arg(format!("/.snapshots/boot/boot-chr{}", snapshot))
                             .output().expect(&format!("Failed to delete chroot snapshot {}", snapshot));
        Command::new("btrfs").args(["sub", "del"])
                             .arg(format!("/.snapshots/etc/etc-chr{}", snapshot))
                             .output().expect(&format!("Failed to delete chroot snapshot {}", snapshot));
        Command::new("btrfs").args(["sub", "del"])
                             .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                             .output().expect(&format!("Failed to delete chroot snapshot {}", snapshot));
        }
}

// Check if inside chroot
pub fn chroot_check() -> bool {
    let read = read_to_string("/proc/mounts").unwrap();
    if read.contains("/.snapshots btrfs") {
        return false;
    } else {
        return true;
    }
}

// Clone tree
pub fn clone_as_tree(snapshot: &str, desc: &str) {
    let i = find_new();
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
        eprintln!("Cannot clone as snapshot {} doesn't exist.", snapshot);
    } else {
        if check_mutability(snapshot) {
            let immutability = "";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
            Command::new("touch").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", i))
                                 .status().unwrap();
        } else {
            let immutability = "-r";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
        }
        append_base_tree(i).unwrap();
        write_tree().unwrap();
        if desc.is_empty() {
            let description = format!("clone of {}", snapshot);
            write_desc(i.to_string().as_str(), &description).unwrap();
        } else {
            let description = desc.split("").collect::<Vec<&str>>().join(" ");
            write_desc(i.to_string().as_str(), &description).unwrap();
        }
        println!("Tree {} cloned from {}.", i,snapshot);
    }
}

// Clone branch under same parent
pub fn clone_branch(snapshot: &str) -> i32 {
    let i = find_new();
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
        eprintln!("Cannot clone as snapshot {} doesn't exist.", snapshot);
    } else {
        if check_mutability(snapshot) {
            let immutability = "";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
            Command::new("touch").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", i))
                                 .status().unwrap();
        } else {
            let immutability = "-r";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
        }
        add_node_to_level(snapshot, i).unwrap();
        write_tree().unwrap();
        let desc = format!("clone of {}", snapshot);
        write_desc(i.to_string().as_str(), &desc).unwrap();
        println!("Branch {} added to parent of {}.", i,snapshot);
    }
    return i;
}

// Recursively clone an entire tree
pub fn clone_recursive(snapshot: &str) {
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
        println!("Cannot clone as tree {} doesn't exist.", snapshot);
    } else {
        let mut children = return_children(snapshot);
        let ch = children.clone();
        children.insert(0, snapshot.to_string());
        let ntree = clone_branch(snapshot);
        let mut new_children = ch.clone();
        new_children.insert(0, ntree.to_string());
        for child in ch {
            let parent = get_parent(&child).unwrap().to_string();
            let index = children.iter().position(|x| x == &parent).unwrap();
            let i = clone_under(&new_children[index], &child);
            new_children[index] = i.to_string();
        }
    }
}

// Clone under specified parent
pub fn clone_under(snapshot: &str, branch: &str) -> i32 {
    let i = find_new();
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
        eprintln!("Cannot clone as snapshot {} doesn't exist.", snapshot);
        } else if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", branch)).try_exists().unwrap() {
        eprintln!("Cannot clone as snapshot {} doesn't exist.", branch);
        } else {
        if check_mutability(snapshot) {
            let immutability = "";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", branch))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", branch))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", branch))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
            Command::new("touch").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", i))
                                 .status().unwrap();
        } else {
            let immutability = "-r";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", branch))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", branch))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", branch))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
        }
        add_node_to_parent(snapshot, i).unwrap();
        write_tree().unwrap();
        let desc = format!("clone of {}", branch);
        write_desc(i.to_string().as_str(), desc.as_str()).unwrap();
        println!("Branch {} added under snapshot {}.", i,snapshot);
    }
    return i;
}

// Add node to branch
pub fn extend_branch(snapshot: &str, desc: &str) {
    let i = find_new();
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
        eprintln!("Cannot branch as snapshot {} doesn't exist.", snapshot);
    } else {
        if check_mutability(snapshot) {
            let immutability = "";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
            Command::new("touch").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", i))
                                 .status().unwrap();
       } else {
            let immutability = "-r";
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                                 .arg(format!("/.snapshots/boot/boot-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                                 .arg(format!("/.snapshots/etc/etc-{}", i)).status().unwrap();
            Command::new("btrfs").args(["sub", "snap"])
                                 .arg(immutability)
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                 .arg(format!("/.snapshots/rootfs/snapshot-{}", i)).status().unwrap();
        }
    }
    add_node_to_parent(snapshot, i).unwrap();
    write_tree().unwrap();
    if desc.is_empty() {
        print!("Branch {} added under snapshot {}.", i,snapshot);
    } else {
        write_desc(i.to_string().as_str(), desc).unwrap();
        print!("Branch {} added under snapshot {}.", i,snapshot);
    }
}

// Find new unused snapshot dir
pub fn find_new() -> i32 {
    let mut i = 0;
    let boots = read_dir("/.snapshots/boot")
        .unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>();
    let etcs = read_dir("/.snapshots/etc")
        .unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>();
    //let vars = read_dir("/.snapshots/var")
        //.unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>(); // Can this be deleted?
    let mut snapshots = read_dir("/.snapshots/rootfs")
        .unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>();
    snapshots.append(&mut etcs.clone());
    //snapshots.append(&mut vars.clone());
    snapshots.append(&mut boots.clone());

    loop {
        i += 1;
        if !snapshots.contains
            (&PathBuf::from(format!("/.snapshots/rootfs/snapshot-{}", i))) && !snapshots
            .contains
            (&PathBuf::from(format!("/.snapshots/etc/etc-{}", i))) && !snapshots
            /*.contains
            (&PathBuf::from(format!("var-{}", i))) && !snapshots*/.contains
            (&PathBuf::from(format!("/.snapshots/boot/boot-{}", i))) {
                break i;
        }
    }
}

// Get current snapshot
pub fn get_current_snapshot() -> String {
    let csnapshot = read_to_string("/usr/share/ash/snap").unwrap();
    csnapshot.trim_end().to_string()
}

// This function returns either empty string or underscore plus name of distro if it was appended to sub-volume names to distinguish
pub fn get_distro_suffix(distro: &str) -> String {
    if distro.contains("ashos") {
        return format!("_{}", distro.replace("_ashos", ""));
    } else {
        std::process::exit(1);
    }
}

// Get tmp partition state
pub fn get_tmp() -> &'static str {
    // By default just return which deployment is running
    let mount_exec = Command::new("cat")
        .args(["/proc/mounts", "|", "grep", "' / btrfs'"])
        .output().unwrap();
    let mount = String::from_utf8_lossy(&mount_exec.stdout).to_string();
    if mount.contains("deploy-aux") {
        let r = "deploy-aux";
        return r;
    } else {
        let r = "deploy";
        return r;
    }
}

// Check EFI
pub fn is_efi() -> bool {
    let is_efi = Path::new("/sys/firmware/efi").try_exists().unwrap();
    is_efi
}

// Make a node mutable
pub fn immutability_disable(snapshot: &str) {
    if snapshot != "0" {
        if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
            eprintln!("Snapshot {} doesn't exist.", snapshot);
        } else {
            if check_mutability(snapshot) {
                println!("Snapshot {} is already mutable.", snapshot);
            } else {
                let excode1 = Command::new("btrfs").arg("property")
                                                   .arg("set")
                                                   .arg("-ts")
                                                   .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                                   .arg("ro")
                                                   .arg("false")
                                                   .status().unwrap();
                let excode2 = Command::new("touch").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", snapshot))
                                                   .status().unwrap();
                if excode1.success() && excode2.success() {
                    println!("Snapshot {} successfully made mutable.", snapshot);
                }
                write_desc(snapshot, " MUTABLE").unwrap();
            }
        }
    } else {
        eprintln!("Snapshot 0 (base) should not be modified.");
    }
}

//Make a node immutable
pub fn immutability_enable(snapshot: &str) {
    if snapshot != "0" {
        if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
            eprintln!("Snapshot {} doesn't exist.", snapshot);
        } else {
            if !check_mutability(snapshot) {
                println!("Snapshot {} is already immutable.", snapshot);
            } else {
                let excode1 = Command::new("rm").arg(format!("/.snapshots/rootfs/snapshot-{}/usr/share/ash/mutable", snapshot))
                                                .status().unwrap();
                let excode2 = Command::new("btrfs").arg("property")
                                                   .arg("set")
                                                   .arg("-ts")
                                                   .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                                                   .arg("ro")
                                                   .arg("true")
                                                   .status().unwrap();
                if excode1.success() && excode2.success() {
                    println!("Snapshot {} successfully made immutable.", snapshot);
                }
                Command::new("sed").arg("-i")
                                   .arg("s|MUTABLE||g")
                                   .arg(format!("/.snapshots/ash/snapshots/{}-desc", snapshot))
                                   .status().unwrap();
            }
        }
    } else {
        eprintln!("Snapshot 0 (base) should not be modified.");
    }
}

// List sub-volumes for the booted distro only
pub fn list_subvolumes() {
    let args = format!("btrfs sub list / | grep -i {} | sort -f -k 9",
                       get_distro_suffix(&detect::distro_id()).as_str());
    Command::new("sh").arg("-c").arg(args).status().unwrap();
}

// Live unlocked shell
pub fn live_unlock() {
    let tmp = get_tmp();
    Command::new("mount").arg("--bind")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                         .arg(format!("/.snapshots/rootfs/snapshot-{}", tmp)).status().unwrap();
    Command::new("mount").arg("--bind")
                         .arg("/etc")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}/etc", tmp)).status().unwrap();
    Command::new("mount").arg("--bind")
                         .arg("/home")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}/home", tmp)).status().unwrap();
    Command::new("mount").arg("--bind")
                         .arg("/tmp")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}/tmp", tmp)).status().unwrap();
    Command::new("mount").arg("--bind")
                         .arg("/var")
                         .arg(format!("/.snapshots/rootfs/snapshot-{}/var", tmp)).status().unwrap();
    Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp)).status().unwrap();
    Command::new("sh").arg("-c")
                        .arg(format!("umount /.snapshots/rootfs/snapshot-{}/*", tmp)).output().unwrap();
    Command::new("umount").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp)).status().unwrap();
}

// Post transaction function, copy from chroot dirs back to read only snapshot dir
pub fn post_transactions(snapshot: &str) {
    let tmp = get_tmp();
    // Some operations were moved below to fix hollow functionality ###
    //File operations in snapshot-chr
    Command::new("rm").arg("-rf")
                      .arg(format!("/.snapshots/boot/boot-chr{}/*", snapshot))
                      .status().unwrap();
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/boot/.", snapshot))
                      .arg(format!("/.snapshots/boot/boot-chr{}", snapshot))
                      .status().unwrap();
    Command::new("rm").arg("-rf")
                      .arg(format!("/.snapshots/etc/etc-chr{}/*", snapshot))
                      .status().unwrap();
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/.", snapshot))
                      .arg(format!("/.snapshots/etc/etc-chr{}", snapshot))
                      .status().unwrap();
    // Keep package manager's cache after installing packages. This prevents unnecessary downloads for each snapshot when upgrading multiple snapshots
    cache_copy(snapshot);
    Command::new("btrfs").args(["sub", "del"])
                         .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                         .status().unwrap();
    Command::new("btrfs").args(["sub", "del"])
                         .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                         .status().unwrap();
    Command::new("btrfs").args(["sub", "del"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                         .status().unwrap();
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-chr{}/usr/share/ash/mutable", snapshot)).try_exists().unwrap() {
        let immutability = "";
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/boot/boot-chr{}", snapshot))
                             .arg(format!("/.snapshots/boot/boot-{}", snapshot)).status().unwrap();
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/etc/etc-chr{}", snapshot))
                             .arg(format!("/.snapshots/etc/etc-{}", snapshot)).status().unwrap();
        // Copy init system files to shared
        init_system_copy(tmp, "post_transactions");
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                             .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                             .status().unwrap();
    } else {
        let immutability = "-r";
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/boot/boot-chr{}", snapshot))
                             .arg(format!("/.snapshots/boot/boot-{}", snapshot)).status().unwrap();
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/etc/etc-chr{}", snapshot))
                             .arg(format!("/.snapshots/etc/etc-{}", snapshot)).status().unwrap();
        // Copy init system files to shared
        init_system_copy(tmp, "post_transactions");
        Command::new("btrfs").args(["sub", "snap"])
                             .arg(format!("{}", immutability))
                             .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                             .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                             .status().unwrap();
    }
    // fix for hollow functionality
    // Unmount in reverse order
    Command::new("umount").arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/resolv.conf", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/dev", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/home", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/proc", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/root", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/run", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/sys", snapshot))
                          .status().unwrap();
    Command::new("umount").arg("-R")
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                          .status().unwrap();
    // Special mutable directories
    let options = snapshot_config_get();
    let mutable_dirs: Vec<&str> = options.get("mutable_dirs")
                                         .map(|dirs| dirs.split(',').filter(|dir| !dir.is_empty()).collect())
                                         .unwrap_or_else(|| Vec::new());
    let mutable_dirs_shared: Vec<&str> = options.get("mutable_dirs_shared")
                                         .map(|dirs| dirs.split(',').filter(|dir| !dir.is_empty()).collect())
                                         .unwrap_or_else(|| Vec::new());
    if !mutable_dirs.is_empty() {
        for mount_path in mutable_dirs {
            Command::new("umount").arg("-R")
                                  .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                  .status().unwrap();
        }
    }
    if !mutable_dirs_shared.is_empty() {
        for mount_path in mutable_dirs_shared {
            Command::new("umount").arg("-R")
                                  .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                  .status().unwrap();
        }
    }
    // fix for hollow functionality
    chr_delete(snapshot);
}

// Prepare snapshot to chroot dir to install or chroot into
pub fn prepare(snapshot: &str) {
    chr_delete(snapshot);
    Command::new("btrfs").args(["sub", "snap"])
                         .arg(format!("/.snapshots/rootfs/snapshot-{}", snapshot))
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                         .status().unwrap();
    // Pacman gets weird when chroot directory is not a mountpoint, so the following mount is necessary
    Command::new("mount").args(["--bind", "--make-slave"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/dev"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/dev", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/home"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/home", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/proc"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/proc", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/root"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/root", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/run"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/run", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/sys"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/sys", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--rbind", "--make-rslave", "/tmp"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/tmp", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave", "/var"])
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var", snapshot))
                         .status().unwrap();
    Command::new("mount").args(["--bind", "--make-slave"])
                         .arg("/etc/resolv.conf")
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/resolv.conf", snapshot))
                         .status().unwrap();
    // File operations for snapshot-chr
    Command::new("btrfs").args(["sub", "snap"])
                         .arg(format!("/.snapshots/boot/boot-{}", snapshot))
                         .arg(format!("/.snapshots/boot/boot-chr{}", snapshot))
                         .status().unwrap();
    Command::new("btrfs").args(["sub", "snap"])
                         .arg(format!("/.snapshots/etc/etc-{}", snapshot))
                         .arg(format!("/.snapshots/etc/etc-chr{}", snapshot))
                         .status().unwrap();
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/boot/boot-chr{}/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/boot", snapshot))
                      .status().unwrap();
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/etc/etc-chr{}/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc", snapshot)).status().unwrap();
    init_system_clean(snapshot, "prepare");
    Command::new("cp").arg("/etc/machine-id")
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/machine-id", snapshot))
                      .status().unwrap();
    Command::new("mkdir").arg("-p")
                         .arg(format!("/.snapshots/rootfs/snapshot-chr{}/.snapshots/ash", snapshot)).status().unwrap();
    Command::new("cp").arg("-f")
                      .arg("/.snapshots/ash/fstree")
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/.snapshots/ash/", snapshot))
                      .status().unwrap();
    // Special mutable directories
    let options = snapshot_config_get();
    let mutable_dirs: Vec<&str> = options.get("mutable_dirs")
                                         .map(|dirs| dirs.split(',').filter(|dir| !dir.is_empty()).collect())
                                         .unwrap_or_else(|| Vec::new());
    let mutable_dirs_shared: Vec<&str> = options.get("mutable_dirs_shared")
                                         .map(|dirs| dirs.split(',').filter(|dir| !dir.is_empty()).collect())
                                         .unwrap_or_else(|| Vec::new());
    if !mutable_dirs.is_empty() {
        for mount_path in mutable_dirs {
            Command::new("mkdir").arg("-p")
                                 .arg(format!("/.snapshots/mutable_dirs/snapshot-{}/{}", snapshot,mount_path))
                                 .status().unwrap();
            Command::new("mkdir").arg("-p")
                                 .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                 .status().unwrap();
            Command::new("mount").arg("--bind")
                                 .arg(format!("/.snapshots/mutable_dirs/snapshot-{}/{}", snapshot,mount_path))
                                 .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                 .status().unwrap();
        }
    }
    if !mutable_dirs_shared.is_empty() {
        for mount_path in mutable_dirs_shared {
            Command::new("mkdir").arg("-p")
                                 .arg(format!("/.snapshots/mutable_dirs/{}", mount_path))
                                 .status().unwrap();
            Command::new("mkdir").arg("-p")
                                 .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                 .status().unwrap();
            Command::new("mount").arg("--bind")
                                 .arg(format!("/.snapshots/mutable_dirs/{}", mount_path))
                                 .arg(format!("/.snapshots/rootfs/snapshot-chr{}/{}", snapshot,mount_path))
                                 .status().unwrap();
        }
    }
}

// Clear all temporary snapshots
pub fn tmp_clear() {
    Command::new("sh").arg("-c")
                        .arg(format!("btrfs sub del /.snapshots/boot/boot-chr*"))
                        .status().unwrap();
    Command::new("sh").arg("-c")
                        .arg(format!("btrfs sub del /.snapshots/etc/etc-chr*"))
                        .status().unwrap();
    Command::new("sh").arg("-c")
                        .arg(format!("btrfs sub del '/.snapshots/rootfs/snapshot-chr*/*'"))
                        .status().unwrap();
    Command::new("sh").arg("-c")
                        .arg(format!("btrfs sub del /.snapshots/rootfs/snapshot-chr*"))
                        .status().unwrap();
}

// Saves changes made to /etc to snapshot
pub fn update_etc() {
    let snapshot = get_current_snapshot();
    let tmp = get_tmp();
    Command::new("btrfs").args(["sub", "del"])
                         .arg(format!("/.snapshots/etc/etc-{}", snapshot)).output().unwrap();
    if check_mutability(&snapshot) {
        let immutability = "";
        Command::new("btrfs").args(["sub", "snap"]).arg(format!("{}", immutability))
                                                   .arg(format!("/.snapshots/etc/etc-{}", tmp))
                                                   .arg(format!("/.snapshots/etc/etc-{}", snapshot)).output().unwrap();
    } else {
        let immutability = "-r";
        Command::new("btrfs").args(["sub", "snap"]).arg(format!("{}", immutability))
                                                   .arg(format!("/.snapshots/etc/etc-{}", tmp))
                                                   .arg(format!("/.snapshots/etc/etc-{}", snapshot)).output().unwrap();
    }
}

// Recursively run an update in tree
/*pub fn update_tree(treename: &str) {
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", treename)).try_exists().unwrap() {
        eprintln!("Cannot update as tree {} doesn't exist.", treename);
    } else {
        //upgrade(treename)
        let mut order = recurse_tree(treename);
        if order.len() > 2 {
            order.remove(0);
            order.remove(0);
        }
        loop {
            if order.len() < 2 {
                break;
            } else {
                let arg = &order[0];
                let sarg = &order[1];
                println!("{}, {}", arg, sarg);
                order.remove(0);
                order.remove(0);
            }
            //auto_upgrade(sarg);
        }
        println!("Tree {} updated.", treename)
    }
}*/

// Write new description (default) or append to an existing one (i.e. toggle immutability)
pub fn write_desc(snapshot: &str, desc: &str) -> std::io::Result<()> {
    let mut descfile = OpenOptions::new().append(true)
                                         .create(true)
                                         .read(true)
                                         .open(format!("/.snapshots/ash/snapshots/{}-desc", snapshot))
                                         .unwrap();
    descfile.write_all(desc.as_bytes()).unwrap();
    Ok(())
}
