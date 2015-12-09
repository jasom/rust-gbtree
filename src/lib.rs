use std::cmp::Ordering::{Greater, Less, Equal};
use std::mem;
use std::ptr;

include! ("lut.rs");

const MAXDEL: usize = 5;

pub struct Node<K, V> {
    key: K,
    value: V,
    children: (Option<Box<Node<K, V>>>, Option<Box<Node<K, V>>>),
}

pub struct GBTreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
    weight: usize,
    deletions: usize,
}

fn floor_log2(mut i: usize) -> usize {
    i>>=1;
    let mut returnme = 0;
    while i != 0 {
        i>>=1;
        returnme +=1;
    }
    returnme
}

unsafe fn swap2<T>(x: *mut T, y: *mut T, x2: *mut T, y2: *mut T) {
    // Give ourselves some scratch space to work with
    let mut tmp: T = mem::uninitialized();

    // Perform the swap
    ptr::copy_nonoverlapping(x, &mut tmp, 1);
    ptr::copy(y, x, 1); // `x` and `y` may overlap
    ptr::copy_nonoverlapping(&tmp, y, 1);

    ptr::copy_nonoverlapping(x2, &mut tmp, 1);
    ptr::copy(y2, x2, 1); // `x` and `y` may overlap
    ptr::copy_nonoverlapping(&tmp, y2, 1);

    // y and t now point to the same thing, but we need to completely forget `tmp`
    // because it's no longer relevant.
    mem::forget(tmp);
}

impl <K, V> Node<K, V> {
    fn _get_weight(node: &Option<Box<Node<K, V>>>) -> usize {
        match node { &None => 1, &Some(ref node) => node.get_weight(), }
    }
    fn get_weight(&self) -> usize {
        Node::_get_weight(&self.children.0) +
            Node::_get_weight(&self.children.1)
    }
    unsafe fn tree_to_vine(root: &mut Node<K, V>) -> usize {
        let mut tail: *mut Node<K, V> = root;
        let mut rest: *mut Option<Box<Node<K, V>>> = &mut (*tail).children.1;
        let mut size: usize = 0;
        loop  {

            /*
            root.print_structure("".to_string());
            println!("");
            */
            match *rest {
                None => return size,
                Some(ref mut in_rest) => {
                    let ptr: *mut Option<Box<Node<K, V>>> =
                        &mut in_rest.children.0;
                    match in_rest.children.0 {
                        None => {
                            size += 1;
                            tail = &mut **in_rest;
                            rest = &mut in_rest.children.1;
                        }
                        Some(ref mut left_contents) => {
                            let left_right: *mut Option<Box<Node<K, V>>> =
                                &mut left_contents.children.1;
                            swap2(ptr, left_right, &mut (*tail).children.1,
                                  left_right);
                            /*
                            ptr::swap(ptr, left_right);
                            ptr::swap(&mut (*tail).children.1,left_right);
                            */
                            rest = &mut (*tail).children.1;
                        }
                    }
                }
            }
        }
    }
    unsafe fn compress(root: &mut Node<K, V>, count: usize) {
        let mut scanner: *mut Node<K, V> = root;
        for _ in 0..count {
            /*
            root.print_structure("compress ".to_string());
            println!("compress");
            println!("");
            */
            Node::leftrot(&mut (*scanner).children.1);
            match (*scanner).children.1 {
                Some(ref mut next) => scanner = &mut **next,
                _ => panic!(),
            }
        }
        /*
        root.print_structure("compress ".to_string());
        println!("compress done");
        println!("");
        */
    }


    unsafe fn vine_to_tree(root: &mut Node<K, V>, mut size: usize) {
        let leaves = size + 1 - (1 << floor_log2(size + 1));
        Node::compress(root, leaves);
        size -= leaves;
        while size > 1 { Node::compress(root, size / 2); size = size / 2; }
    }
    unsafe fn leftrot(node: *mut Option<Box<Node<K, V>>>) {
        match *node {
            Some(ref mut n) => {
                let right_child: *mut Option<Box<Node<K, V>>> =
                    &mut n.children.1;
                match *right_child {
                    Some(ref mut n) => {
                        let right_left_child: *mut Option<Box<Node<K, V>>> =
                            &mut n.children.0;

                        /*
                        ptr::swap(node,right_child);
                        ptr::swap(right_child,right_left_child);
                        */
                        swap2(node, right_child, right_child,
                              right_left_child);
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[cfg(feature = "print_structure")]
    fn print_structure(&self, indent: String) {
        match self.children.0 {
            Some(ref child) => {
                println!("{}Left: {:p}" , indent , child);
                child.print_structure(indent.to_string() + " ");
            }
            None => println!("{}Left: O" , indent),
        }
        match self.children.1 {
            Some(ref child) => {
                println!("{}Rght: {:p}" , indent , child);
                child.print_structure(indent + " ");
            }
            None => println!("{}Rght: O" , indent),
        }
    }
    fn rebuild(&mut self) {

        /*
        self.print_structure("rebalance start ".to_string());
        println!("");
        */
        unsafe {
            let mut swap_back: Node<K, V> = mem::uninitialized();
            mem::swap(self, &mut swap_back);
            let mut root =
                Node{key: mem::uninitialized(),
                     value: mem::uninitialized(),
                     children: (None, Some(Box::new(swap_back))),};
            let size = Node::tree_to_vine(&mut root);
            Node::vine_to_tree(&mut root, size);
            swap_back = *root.children.1.unwrap();
            mem::swap(&mut swap_back, self);
            mem::forget(swap_back);
            mem::forget(root.key);
            mem::forget(root.value);
        }
        /*
        self.print_structure("rebalance_end ".to_string());
        println!("");
        */
    }
}



impl <K: Ord + Copy + PartialEq + PartialOrd, V> GBTreeMap<K, V> {
    pub fn new() -> GBTreeMap<K, V> {
        GBTreeMap{root: None, weight: 1, deletions: 0,}
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut cur_node: *mut Option<Box<Node<K, V>>> = &mut self.root;
        let mut depth = 0;
        unsafe {
            loop  {
                match *cur_node {
                    None => {
                        *cur_node =
                            Some(Box::new(Node{key: key,
                                               value: value,
                                               children: (None, None),}));
                        self.weight += 1;
                        if self.weight < MINWEIGHT[depth] {
                            self.fix_balance(key, depth);
                        }
                        return None;
                    }
                    Some(ref mut node) => {
                        depth += 1;
                        match key.cmp(&node.key) {
                            Less => cur_node = &mut node.children.0,
                            Greater => cur_node = &mut node.children.1,
                            Equal =>
                            return Some(mem::replace(&mut node.value, value)),
                        }
                    }
                }
            }
        }
    }

    pub fn fix_balance(&mut self, key: K, d1: usize) {
        unsafe {
            let mut stack: Vec<*mut Node<K, V>> =
                Vec::with_capacity(MAXDEPTH);
            let mut p: *mut Option<Box<Node<K, V>>> = &mut self.root;
            for _ in 0..d1 {
                match *p {
                    Some(ref mut pnode) => {
                        p =
                            if key < pnode.key {
                                &mut pnode.children.0
                            } else { &mut pnode.children.1 };
                        stack.push(&mut **pnode);
                    }
                    None => panic!(),
                }
            }
            let mut w = 2;
            let mut d2 = stack.len() - 1;
            loop  {
                d2 -= 1;
                match (*stack[d2]).children.0 {
                    Some(ref child) if
                    ((&**child) as *const Node<K, V>) ==
                        (stack[d2 + 1] as *const Node<K, V>) =>
                    w += Node::_get_weight(&(*stack[d2]).children.1),
                    _ => w += Node::_get_weight(&(*stack[d2]).children.0),
                }
                if w < MINWEIGHT[d1 - d2] { return (*stack[d2]).rebuild(); }
            }
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        unsafe {
            let mut t: *mut Option<Box<Node<K, V>>> = &mut self.root;
            let mut candidate: *mut Option<Box<Node<K, V>>> = &mut None;
            let mut last: *mut Option<Box<Node<K, V>>> = t;
            loop  {
                match *t {
                    None =>
                    match *candidate {
                        Some(ref mut cnode) =>
                        match *last {
                            Some(ref mut lnode) => {
                                if *key == cnode.key {
                                    let mut result_node = None;
                                    if last == candidate {
                                        let left_child:
                                                *mut Option<Box<Node<K, V>>> =
                                            &mut cnode.children.0;
                                        swap2(last, &mut result_node, last,
                                              left_child);
                                        /*
                                        ptr::swap(last,&mut result_node);
                                        ptr::swap(last,left_child);
                                        */
                                        if self.deletions >
                                               MAXDEL * self.weight {
                                            match self.root {
                                                Some(ref mut rnode) =>
                                                rnode.rebuild(),
                                                _ => (),
                                            }
                                            self.deletions = 0;
                                        }

                                        return Some(result_node.unwrap().value);
                                    }
                                    /* candidate != last */
                                    let right_child:
                                            *mut Option<Box<Node<K, V>>> =
                                        &mut lnode.children.1;
                                    mem::swap(&mut lnode.key, &mut cnode.key);
                                    mem::swap(&mut lnode.value,
                                              &mut cnode.value);
                                    swap2(last, &mut result_node, last,
                                          right_child);
                                    /*
                                    ptr::swap(last,&mut result_node);
                                    ptr::swap(last,right_child);
                                    */
                                    return Some(result_node.unwrap().value);
                                }
                                return None
                            }
                            _ => panic!(),
                        },
                        _ => return None,
                    },
                    Some(ref mut node) => {
                        last = t;

                        //println!("{}",node.key);
                        if key < &node.key {
                            //println!("Left");
                            t = &mut node.children.0;
                        } else {
                            //println!("Right");
                            candidate = t;
                            t = &mut node.children.1;
                        }
                    }
                }
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut cur_node = &self.root;
        loop  {
            match cur_node {
                &None => return None,
                &Some(ref node) =>
                match key.cmp(&node.key) {
                    Less => cur_node = &node.children.0,
                    Greater => cur_node = &node.children.1,
                    Equal => return Some(&node.value),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use test::Bencher;
    #[test]
    fn insert_once() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        x.insert(1, 2);
    }

    #[test]
    fn find_once() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        x.insert(1, 2);
        assert_eq!(x . get ( & 1 ) , Some ( & 2 ));
    }

    #[test]
    fn find_thrice() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        x.insert(4, 6);
        x.insert(3, 5);
        x.insert(1, 2);
        assert_eq!(x . get ( & 1 ) , Some ( & 2 ));
        assert_eq!(x . get ( & 3 ) , Some ( & 5 ));
        assert_eq!(x . get ( & 4 ) , Some ( & 6 ));
    }

    #[test]
    fn test_floor_log2() {
        for i in 1..65536 {
            assert_eq!(super:: floor_log2 ( i ) , ( i as f64 ) . log ( 2.0 ) .
                       floor (  ) as usize);
        }
    }
    #[test]
    fn delete_lots() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        for i in 0..100000 { x.insert(i, i); }
        for i in 0..100000 {
            assert_eq!(x . remove ( & i ) , Some ( i ));
            assert_eq!(x . remove ( & i ) , None);
        }
    }
    #[test]
    fn insert_lots() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        for i in (0..100000).rev() { x.insert(i, 1000000 - i); }

        for i in 0..100000 {
            assert_eq!(x . get ( & i ) , Some ( & ( 1000000 - i ) ));
        }
    }

    #[test]
    fn insert_lots2() {
        let mut x: GBTreeMap<u32, u32> = GBTreeMap::new();
        for i in 0..100000 { x.insert(i, i); }
    }

    /*
    #[bench]
    fn bench_1000000_inserts(b :&mut Bencher) {
        b.iter(|| {
            let n = test::black_box(1000000);
            let mut x : GBTreeMap<u32,u32> = GBTreeMap::new();
            for i in 0..n {
                x.insert(i,i);
            }
        })
    }
    */
}
