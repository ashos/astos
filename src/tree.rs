use crate::get_current_snapshot;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3::{PyErr, PyObject};
use std::fs::{File, OpenOptions, read_to_string};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;


// Clone within node
pub fn add_node_to_level(tree: &PyObject, id: &str, val: i32) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Get parent
        let npar = get_parent(&tree, id)?;
        let npar_dict = PyDict::new_bound(py);
        npar_dict.set_item("npar", npar)?;

        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Filter as kwarg
        let filter = py.eval_bound("lambda node: ('x'+str(node.name)+'x') in ('x'+str(npar)+'x')", Some(&npar_dict), None)?;
        let filter_ = PyDict::new_bound(py);
        filter_.set_item("filter_", filter)?;

        // Parent value
        let par = anytree.call_method("find", (tree,), Some(&filter_))?;

        // Parent as kwarg
        let parent = PyDict::new_bound(py) ;
        parent.set_item("parent", par)?;

        // Node value
        let node: PyObject = anytree.call_method("Node", (val,), Some(&parent))?.extract()?;

        Ok(node)
    })
}

// Add child to node
pub fn add_node_to_parent(tree: &PyObject, id: &str, val: i32) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Set id
        let id_dict = PyDict::new_bound(py);
        id_dict.set_item("id", id)?;

        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Filter as kwarg
        let filter = py.eval_bound("lambda node: ('x'+str(node.name)+'x') in ('x'+str(id)+'x')", Some(&id_dict), None)?;
        let filter_ = PyDict::new_bound(py);
        filter_.set_item("filter_", filter)?;

        // Parent value
        let par = anytree.call_method("find", (tree,), Some(&filter_))?;

        // Parent as kwarg
        let parent = PyDict::new_bound(py) ;
        parent.set_item("parent", par)?;

        // Node value
        let node: PyObject = anytree.call_method("Node", (val,), Some(&parent))?.extract()?;

        Ok(node)
    })
}

// Add to root tree
pub fn append_base_tree(tree: &PyObject, val: i32) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Parent as kwarg
        let parent = PyDict::new_bound(py) ;
        parent.set_item("parent", tree.getattr(py, "root")?)?;

        let node: PyObject = anytree.call_method("Node", (val,), Some(&parent))?.extract()?;

        Ok(node)
    })
}

// Import fstree file
pub fn fstree() -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Import DictImporter and call import_ function
        let importer = py.import_bound("anytree.importer")?;
        let dict_importer = importer.getattr("DictImporter")?;
        let importer_instance = dict_importer.call(PyTuple::empty_bound(py), None)?;

        // Import tree file
        let tree_file = import_tree_file("/.snapshots/ash/fstree")?;

        // Call import_ function with tree_file argument
        let fstree: PyObject = importer_instance.call_method("import_", (tree_file,), None)?.extract()?;

        Ok(fstree)
    })
}

// Get parent
pub fn get_parent(tree: &PyObject, id: &str) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Set id
        let id_dict = PyDict::new_bound(py);
        id_dict.set_item("id", id)?;

        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Filter as kwarg
        let filter = py.eval_bound("lambda node: ('x'+str(node.name)+'x') in ('x'+str(id)+'x')", Some(&id_dict), None)?;
        let filter_ = PyDict::new_bound(py);
        filter_.set_item("filter_", filter)?;

        // Parent value
        let anytree_call = anytree.call_method("find", (tree,), Some(&filter_))?;
        let par: PyObject = anytree_call.getattr("parent")?.getattr("name")?.extract()?;

        Ok(par)
    })
}

// Import filesystem tree file
fn import_tree_file(treename: &str) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Import ast python module
        let ast = py.import_bound("ast")?;

        // Read first line in tree file
        let treefile = File::open(treename)?;
        let buf_read = BufReader::new(treefile);
        let mut read = buf_read.lines();
        let treefile_readline = read.next().unwrap()?;

        // Use literal_eval from ast
        let tree_file: PyObject = ast.getattr("literal_eval")?.call((treefile_readline,), None)?.extract()?;

        Ok(tree_file)
    })
}

// Return order to recurse tree
pub fn recurse_tree(tree: &PyObject, cid: &str) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    for child in return_children(&tree, cid).unwrap() {
        let par = get_parent(&tree, &child).unwrap().to_string();
        if child != cid {
            order.push(par);
            order.push(child);
        }
    }
    order
}

// Remove node from tree
pub fn remove_node(tree: &PyObject, id: &str) -> Result<PyObject, PyErr> {
    Python::with_gil(|py| {
        // Set id
        let id_dict = PyDict::new_bound(py);
        id_dict.set_item("id", id)?;

        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Filter as kwarg
        let filter = py.eval_bound("lambda node: ('x'+str(node.name)+'x') in ('x'+str(id)+'x')", Some(&id_dict), None)?;
        let filter_ = PyDict::new_bound(py);
        filter_.set_item("filter_", filter)?;

        // Parent value
        let parent: Option<String> = None;
        let anytree_call = anytree.call_method("find", (tree,), Some(&filter_))?;
        anytree_call.setattr("parent", parent)?;
        let par: PyObject = anytree_call.getattr("parent")?.extract()?;

        Ok(par)
    })
}

// Return all children for node
pub fn return_children(tree: &PyObject, id: &str) -> Result<Vec<String>, PyErr> {
    Python::with_gil(|py| {
        // Set some values
        let mut children: Vec<String> = Vec::new();
        let id_dict = PyDict::new_bound(py);
        id_dict.set_item("id", id)?;

        // Import anytree
        let anytree =  py.import_bound("anytree")?;

        // Filter as kwarg
        let filter = py.eval_bound("lambda node: ('x'+str(node.name)+'x') in ('x'+str(id)+'x')", Some(&id_dict), None)?;
        let filter_ = PyDict::new_bound(py);
        filter_.set_item("filter_", filter)?;

        // Parent value
        let par = anytree.call_method("find", (tree,), Some(&filter_))?;

        // Import PreOrderIter
        let preorderiter = anytree.call_method("PreOrderIter", (par,), None)?.iter();

        for child in preorderiter? {
            children.push(child?.getattr("name").unwrap().to_string());
        }
        if let Some(index) = children.iter().position(|x| x == id) {
            children.remove(index);
        }
        Ok(children)
    })
}

// Print out tree with descriptions
pub fn tree_print(tree: &PyObject) {
    Python::with_gil(|py| {
        let snapshot = get_current_snapshot();

        // From anytree import AsciiStyle, RenderTree
        let anytree =  py.import_bound("anytree").unwrap();
        let asciistyle = anytree.call_method("AsciiStyle", PyTuple::empty_bound(py), None).unwrap();
        let style = PyDict::new_bound(py);
        style.set_item("style", asciistyle).unwrap();
        let rendertree = anytree.call_method("RenderTree", (tree,), Some(&style)).unwrap();

        for row in rendertree.iter().unwrap() {
            let node = row.as_ref().unwrap().getattr("node").unwrap();
            if Path::new(&format!("/.snapshots/ash/snapshots/{}-desc", node.getattr("name").unwrap().to_string())).is_file() {
                let desc = read_to_string(format!("/.snapshots/ash/snapshots/{}-desc", node.getattr("name").unwrap().to_string())).unwrap();
                if snapshot != node.getattr("name").unwrap().to_string() {
                    println!("{}{} - {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                } else {
                    println!("{}{}*- {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                }
            } else if node.getattr("name").unwrap().to_string() == "0" {
                let desc = "base snapshot.";
                if snapshot != node.getattr("name").unwrap().to_string() {
                    println!("{}{} - {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                } else {
                    println!("{}{}*- {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                }
            } else if node.getattr("name").unwrap().to_string() == "root" {
                let desc = "";
                if snapshot != node.getattr("name").unwrap().to_string() {
                    println!("{}{} {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                } else {
                    println!("{}{} {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                }
            } else {
                let desc = "";
                if snapshot != node.getattr("name").unwrap().to_string() {
                    println!("{}{} - {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                } else {
                    println!("{}{}*- {}", row.unwrap().getattr("pre").unwrap().to_string(), node.getattr("name").unwrap().to_string(), desc);
                }
            }
        }
    })
}

// Save tree to file
pub fn write_tree(tree: &PyObject) -> Result<(), PyErr> {
    Python::with_gil(|py| {
        // Import DictExporter
        let exporter = py.import_bound("anytree.exporter")?;
        let dict_exporter = exporter.getattr("DictExporter")?;
        let exporter_instance = dict_exporter.call(PyTuple::empty_bound(py), None)?;

        // Open & edit tree file
        let fstreepath = "/.snapshots/ash/fstree";
        let mut fsfile = OpenOptions::new().read(true)
                                           .write(true)
                                           .truncate(true)
                                           .open(fstreepath)?;

        // Call export function with fstree argument
        let to_write = exporter_instance.call_method("export", (tree,), None);
        let write = fsfile.write_all(to_write.unwrap().to_string().as_bytes())?;
        Ok(write)
    })
}
