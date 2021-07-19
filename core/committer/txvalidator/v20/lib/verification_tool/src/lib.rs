#![allow(warnings, unused)]

use std::time::{Instant};

use std::thread;

use std::sync::{Arc, Mutex};
use std::sync::Barrier;
use std::sync::atomic::{AtomicI64, Ordering, AtomicBool};

use rand::Rng;

use std::collections::BTreeMap;
//use std::collections::linked_list;
use std::collections::LinkedList;
use std::collections::HashMap;
use std::collections::HashSet;

use chashmap::CHashMap;

//use cty::c_int;
use libc::c_int;
use libc::c_char;
use libc::c_long;

use serde::{Deserialize, Serialize};
use serde_json::Result;

// TODO: should make these a command-line argument
const NUM_TOTAL_THRDS: u64 = 8;
const NUM_THRDS: u64 = NUM_TOTAL_THRDS - 1;
static TEST_SIZE: u64 = 10;

static INT_MIN: i64 = i64::MIN;
static INT_MAX: i64 = i64::MAX;
static LONG_MAX: i64 = i64::MAX;


#[derive(PartialEq, Copy, Clone)]
enum Status
{
    PRESENT,
    ABSENT
}

#[derive(PartialEq, Copy, Clone)]
enum Semantics
{
    FIFO,
    LIFO,
    SET,
    MAP,
    PRIORITY
}

#[derive(PartialEq, Copy, Clone)]
enum MethodType // was "type" in the original code, but rust uses "type" as a keyword
{
    PRODUCER,
    CONSUMER,
    READER,
    WRITER
}

#[derive(Serialize, Deserialize)]
struct event{
    from: String,
    to: String,
    value: i32,
}

struct Transaction
{
    amount: i64,
    send_addr: i64, // NOTE: I think for actual blockchains, wallets have separate sender and receiver addresses, but for our simulation, we'll just treat them as if they are the same.
    receive_addr: i64,
}

impl Transaction
{
    fn from_event(e: event, sender_id: i64, receiver_id: i64) -> Self
    {
        Transaction
        {
            amount: e.value as i64, // for some reason, this is gets decoded as an i32. TODO?
            send_addr: sender_id,
            receive_addr: receiver_id
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
struct Method
{
    id: i64,
    process: i64,
    //int item;
    item_key: i64,
    item_val: i64,
    semantics: Semantics,
    MethodType: MethodType, // MethodType was originally called type, but this is a keyword in Rust
    invocation: i64,
    response: i64,
    quiescent_period: i64,
    status: bool
}

impl Method
{
    fn Method(_id: i64, _process: i64, _item_key: i64, _item_val: i64, _semantics: Semantics, _MethodType: MethodType, _invocation: i64, _response: i64, _status: bool) -> Self// TODO: change name? can structs have methods named the same thing?
    {
        Method
        {
            id : _id,
            process : _process,
            item_key : _item_key,
            item_val : _item_val,
            semantics : _semantics,
            MethodType : _MethodType,
            invocation : _invocation,
            response : _response,
            quiescent_period : -1,
            status : _status
        }
    }
}

#[derive(PartialEq, Clone)]
struct Item
{
    key: i64,
    value: i64,
    sum: f64,

    numerator: i64,
    denominator: i64,

    exponent: f64,

    status: Status,

    promote_items: Vec<i64>, // rust's equivalent of a stack (use push_front() to push and pop_front() to pop. front() to peek.)
    demote_methods: Vec<Method>,
    producer: i64,//btree_map::Iter<'a, i64, Method>, // you'll have to call the iterator for this later. the original code put the iterator as the struct field, but that's too hard in rust.

    sum_f: f64,
    numerator_f: i64,
    denominator_f: i64,
    exponent_f: f64,

    sum_r: f64,
    numerator_r: i64,
    denominator_r: i64,
    exponent_r: f64
}

impl Item
{
    fn newFromKey(_key: i64) -> Item
    {
        Item
        {
            key : _key,
            value : INT_MIN,
            sum : 0.0,
            numerator : 0,
            denominator : 1,
            exponent : 0.0,
            status : Status::PRESENT,

            promote_items: Vec::new(),
            demote_methods: Vec::new(),
            producer: 0,

            sum_f : 0.0,
            numerator_f : 0,
            denominator_f : 0,
            exponent_f : 0.0,
            sum_r : 0.0,
            numerator_r : 0,
            denominator_r : 1,
            exponent_r : 0.0
        }
    }

    // TODO: rename this (rust doesn't allow overloading)
    fn newFromVal(_key: i64, _val: i64) -> Item
    {
        Item
        {
            key : _key,
            value : _val,
            sum : 0.0,
            numerator : 0,
            denominator : 1,
            exponent : 0.0,
            status : Status::PRESENT,

            promote_items: Vec::new(), // rust's equivalent of a stack
            demote_methods: Vec::new(),
            producer: 0,

            sum_f : 0.0,
            numerator_f : 0,
            denominator_f : 1,
            exponent_f : 0.0,
            sum_r : 0.0,
            numerator_r : 0,
            denominator_r : 1,
            exponent_r : 0.0
        }
    }

    fn add_int(&mut self, x: i64)
    {
        let add_num: i64 = x * self.denominator;
        self.numerator += add_num;
        self.sum = (self.numerator as f64) / (self.denominator as f64);
    }

    fn sub_int(&mut self, x: i64)
    {
        let sub_num: i64 = x * self.denominator;
        self.numerator -= sub_num;
        self.sum = (self.numerator as f64) / (self.denominator as f64);
    }

    fn add_frac(&mut self, num: i64, den: i64)
    {
        if self.denominator % den == 0
        {
            self.numerator += num * self.denominator / den;
        }
        else if den % self.denominator == 0
        {
            self.numerator = self.numerator * den / self.denominator + num;
            self.denominator = den;
        }
        else
        {
            self.numerator = self.numerator * den + self.denominator * num;
            self.denominator *= den;
        }
        self.sum = (self.numerator as f64) / (self.denominator as f64);
    }

    fn sub_frac(&mut self, num: i64, den: i64)
    {
        if self.denominator % den == 0
        {
            self.numerator -= num * self.denominator / den;
        }
        else if den % self.denominator == 0
        {
            self.numerator = self.numerator * den / self.denominator - num;
            self.denominator = den;
        }
        else
        {
            self.numerator = self.numerator * den - self.denominator * num;
            self.denominator *= den;
        }
        self.sum = (self.numerator as f64) / (self.denominator as f64);
    }

    fn demote(&mut self)
    {
        self.exponent += 1.0;
        let den: i64 = i64::pow(2, self.exponent as u32);
        self.sub_frac(1, den);
    }

    fn promote(&mut self)
    {
        let den: i64 = i64::pow(2, self.exponent as u32);
        self.add_frac(1, den);
        self.exponent -= 1.0;
    }

    /*fn add_frac_f(&mut self, num: i64, den: i64)
    {
        if (self.denominator_f % den) == 0
        {
            self.numerator_f += num * self.denominator_f / den;
        }
        else if (den % self.denominator_f) == 0
        {
            self.numerator_f = self.numerator_f * den / self.denominator_f + num;
            self.denominator_f = den;
        }
        else
        {
            self.numerator_f = self.numerator_f * den + num * self.denominator_f;
            self.denominator_f = self.denominator_f * den;
        }
        self.sum_f = (self.numerator_f as f64) / (self.denominator_f as f64);
    }*/

    fn sub_frac_f(&mut self, num: i64, den: i64)
    {
        if (self.denominator_f % den) == 0
    {
            self.numerator_f -= num * self.denominator_f / den;
        }
        else if (den % self.denominator_f) == 0
        {
            self.numerator_f = self.numerator_f * den / self.denominator_f - num;
            self.denominator_f = den;
        }
        else
        {
            self.numerator_f = self.numerator_f * den - num * self.denominator_f;
            self.denominator_f = self.denominator_f * den;
        }
        self.sum_f = (self.numerator_f as f64) / (self.denominator_f as f64);
    }

    fn demote_f(&mut self)
    {
        self.exponent_f += 1.0;
        let den: i64 = i64::pow(2, self.exponent_f as u32);
        self.sub_frac_f(1, den);
    }

    /*fn promote_f(&mut self)
    {
        let den: i64 = i64::pow(2, self.exponent_f as u32);
        self.sub_frac_f(1, den);
        self.exponent_f -= 1.0;
    }*/

    /*fn add_frac_r(&mut self, num: i64, den: i64)
    {
        if (self.denominator_r % den) == 0
        {
            self.numerator_r += num * self.denominator_r / den;
        }
        else if (den % self.denominator_r) == 0
        {
            self.numerator_r = self.numerator_r * den / self.denominator_r + num;
            self.denominator_r = den;
        }
        else
        {
            self.numerator_r = self.numerator_r * den + num * self.denominator_r;
            self.denominator_r = self.denominator_r * den;
        }
        self.sum_r = (self.numerator_r as f64) / (self.denominator_r as f64);
    }*/

    /*fn sub_frac_r(&mut self, num: i64, den: i64)
    {
        if (self.denominator_r % den) == 0
        {
            self.numerator_r -= num * self.denominator_r / den;
        }
        else if (den % self.denominator_r) == 0
        {
            self.numerator_r = self.numerator_r * den / self.denominator_r - num;
            self.denominator_r = den;
        }
        else
        {
            self.numerator_r = self.numerator_r * den - num * self.denominator_r;
            self.denominator_r = self.denominator_r * den;
        }
        self.sum_r = (self.numerator_r as f64) / (self.denominator_r as f64);
    }*/

    fn demote_r(&mut self)
    {
        self.exponent_r += 1.0;
        let den: i64 = i64::pow(2, self.exponent_r as u32);
        self.sub_frac_f(1, den);
    }

    /*fn promote_r(&mut self)
    {
        let den: i64 = i64::pow(2, self.exponent_r as u32);
        self.sub_frac_r(1, den);
        self.exponent_r -= 1.0;
    }*/
}

/*struct Block
{
    start: i64,
    finish: i64
}

impl Block
{
    fn new() -> Block
    {
        Block
        {
            start : 0,
            finish : 0,
        }
    }
}*/

/*
// each thread runs this function to modify the shared map
// txn_methods = execute_txns(all_txn, account_balances);
fn execute_txns(all_txn: &mut Vec<Method>, account_balances: Arc<CHashMap<i64, i64>>)
{
    let mut method_id = thread_id;
    //println!("meta: thread {} is waiting at barrier", method_id);
    barrier.wait();
    for i in 0..TEST_SIZE
    {
        let mut item_key: i64;
        let mut item_val: i64 = -1;
        let res: bool = true;
        let op_dist = rand::thread_rng().gen_range(0..100);
        //println!("meta: random value is {}", op_dist);

        // we set worker_type to be CONSUMER, PRODUCER, or READER depending on the random number we got
        let worker_type: MethodType;


        let invocation = epoch.elapsed().as_nanos();

        if op_dist <= 50
        {
            worker_type = MethodType::CONSUMER;
            let item_erase = op_dist + 1;//(method_id as i64) - (2 * (NUM_THRDS as i64));
            //println!("item_erase is {}", item_erase);

            // tbb::concurrent_hash_map.erase() takes a key, and removes it from the hashmap
            // the rust equivalent is remove(). curiously, it takes a reference, not a value.

            if map.contains_key(&item_erase)
            {
                {
                    println!("map configuration before removing {}: {:?}", &item_erase, map);
                }
                println!("removing key from Hashmap {}", &item_erase);
                map.remove(&item_erase);
                {
                    println!("map configuration after removing {}: {:?}", &item_erase, map);
                }
            }
            else
            {
                // for demo:
                // this section shows a (poor) attempt at concurrency
                // the idea here is that by checking that the item is in the hashmap before removing it,
                // there shouldn't be any failed removals.

                // however, other threads are acting on the hashmap between the check to see that the element is in the hashmap, and the actual removal of that element.
                // this is why sometimes, the concurrent history is correct, and sometimes it isn't.
                continue;
            }




            if res
            {
                item_key = item_erase;
            }
            else
            {
                item_key = INT_MIN;
            }
        }
        else
        {
            worker_type = MethodType::PRODUCER;
            item_key = method_id as i64; // reminder, item_key and item_value are just dummy values from the old hashmap demo
            item_val = method_id as i64;
            map.insert(item_key, item_val);
            println!("adding key to Hashmap {}", item_key);
        }
        /*else
        {
            worker_type = MethodType::READER;
            item_key = (method_id as i64) - (NUM_THRDS as i64);

            if map.contains_key(&item_key)
            {
                item_val = *map.get(&item_key).unwrap();
            }
            else
            {
                item_key = INT_MIN;
                item_val = INT_MIN;
            }
        }*/

        let response = epoch.elapsed().as_nanos();

        let m1 = Method::Method(method_id as i64, thread_id as i64, item_key, item_val, Semantics::MAP, worker_type, invocation as i64, response as i64, res);

method_id += NUM_THRDS;


        {   // acquire the lock on the method list so we can add this method to it
            let mut thrd_list = thrd_lists.lock().unwrap();
            thrd_list[thread_id as usize].push_back(m1);
        }   // the lock is released at the end of this scope we created


        // the C++ code keeps an array called thrd_lists_size that stores the size of each linked list.
        // this is because the c++ version of thrd_lists was an array, which doesn't keep track of length. rust's LinkedList does.
        // in rust, we can just call thrd_list.len().

    }
    //println!("map len at end:{}", concurrent_map.len());
    //println!("thread_number is: {}", method_id);
    done.store(thread_id as usize, true);
}*/



// TODO: map_iterator and vector_items don't need to be mutable. we don't write to them.
fn handle_failed_consumer(map_methods: &BTreeMap<i64, Method>, vector_items: &mut Vec<Item>, it: i64, vec_item: &mut Item, stack_failed: &mut Vec<Item>)
{
    let mut keys = Vec::new();
    {
        for k in map_methods.keys()
        {
            keys.push(k);
            //println!("pushed key: {}", k)
        }
    }
    // iterates through the map_methods until we reach the element at the passed-in iterator.
    for it_0 in map_methods.iter()
    {
        if it_0.0 == keys[it as usize]
        {
            break; // ends the loop (and therefore the function)
        }

        if it_0.1.response < map_methods[keys[it as usize] ].invocation
        {

            let vec_item_0;

            if map_methods[keys[it as usize] ].item_key != INT_MIN
            {
                vec_item_0 = vector_items[it_0.1.item_key as usize].clone();
            }
            else
            {
                vec_item_0 = vector_items[(TEST_SIZE * NUM_THRDS) as usize].clone();
            }

            if (it_0.1.MethodType == MethodType::PRODUCER) && (vec_item.status == Status::PRESENT) && (it_0.1.semantics == Semantics::FIFO || it_0.1.semantics == Semantics::LIFO || map_methods[keys[it as usize] ].item_key == it_0.1.item_key)
            {
                // this is the vector equivalent of a linked list's push_front()
                stack_failed.insert(0, vec_item_0);
            }
        }
    }
}


// TODO: map_iterator and vector_items don't need to be mutable. we don't write to them.
fn handle_failed_read(map_methods: & BTreeMap<i64, Method>, vector_items: &mut Vec<Item>, it: i64, vec_item: &mut Item, stack_failed: &mut Vec<Item>)
{
    let mut keys = Vec::new();
    {
        for k in map_methods.keys()
        {
            keys.push(k);
            //println!("pushed key: {}", k)
        }
    }

    // iterates through the map_methods until we reach the element at the passed-in iterator.
    for it_0 in map_methods.iter()
    {
        if it_0.0 == keys[it as usize]
        {
            break; // ends the loop (and therefore the function)
        }

        if it_0.1.response < map_methods[keys[it as usize] ].invocation
        {

            let vec_item_0;

            if map_methods[keys[it as usize] ].item_key != INT_MIN
            {
                vec_item_0 = vector_items[it_0.1.item_key as usize].clone();
            }
            else
            {
                vec_item_0 = vector_items[(TEST_SIZE * NUM_THRDS) as usize].clone();
            }

            if (it_0.1.MethodType == MethodType::PRODUCER) && (vec_item.status == Status::PRESENT) && (map_methods[keys[it as usize] ].item_key == it_0.1.item_key)
            {
                // this is the vector equivalent of a linked list's push_front()
                stack_failed.insert(0, vec_item_0);
            }
        }
    }
}


fn verify(account_balances: &mut HashMap<i64, i64>, all_methods: &mut LinkedList<Method>, method_count: &mut u64) -> bool
{

    let mut map_methods: BTreeMap<i64, Method> = BTreeMap::<i64, Method>::new();

    let mut vector_items: Vec<Item>;
    vector_items = vec![Item::newFromKey(INT_MAX); (TEST_SIZE * NUM_THRDS) as usize + 1];

    let mut it_start: i64 = 0; // TODO: should this be zero? this is just a guess
    let mut count_iterated = 0;

    //let mut method_count: u64 = 0; // TODO: the original code declared this in global, and initialized to 0 in main. do we need to do this?

    let mut outcome = false;

    // the core verification logic is in this loop, and the verify_checkpoint() function it calls
    loop
    {
        let mut min: i64 = INT_MIN;

            let mut response_time: i64 = 0;

            { // we create this scope to minimize the amount of time the lock on thrd_lists is acquired
                //let m = iter.next().unwrap();
                for method in all_methods.iter()
                {

                    // we are copying the value here. TODO: do we need to deal with the actual object? rust won't like this.
                    let mut m: Method = *method;

                    //println!("&m.response is {}", &m.response);
                    let mut it_method = map_methods.get(&m.response);
                    // this is just a really long and convoluted way to check that the iterator hasn't reached the end of the BTreeMap.
                    // BTreeMap has a method for getting the last element, but it's only available in nightly releases of the rust compiler.
                    while it_method != None
                    {
                        m.response += 1;
                        it_method = map_methods.get(&m.response);
                    }
                    //let map_iter = map_methods.iter();

                    response_time = m.response;


                    map_methods.insert(m.response, m);
                }

            }

            if response_time < min
            {
                min = response_time;
            }



        if verify_checkpoint(account_balances, &mut map_methods, &mut it_start, &mut count_iterated, min, &mut true, method_count, &mut vector_items) == false
        {
            return false;
        }
        else
        {
            return true;
        }
    }
    //verify_checkpoint(&mut map_methods, &mut it_start, &mut count_iterated, LONG_MAX, &mut false, &mut method_count, &mut vector_items, final_outcome.clone());
}


fn verify_checkpoint(account_balances: &mut HashMap<i64, i64>, map_methods: &mut BTreeMap<i64, Method>, it_start: &mut i64, count_iterated: &mut u64, min: i64, reset_it_start: &mut bool, method_count: &mut u64, vector_items: &mut Vec<Item>) -> bool
{
    let mut final_outcome: bool = false; // TODO: does the default value matter here?
    //let mut map_methods_1 = Rc::clone(map_methods_0);
    //let mut map_methods = Rc::get_mut(&mut map_methods_1).unwrap();
    let mut keys = Vec::new();
    let map_methods_clone = map_methods.clone();
    {

        for k in map_methods_clone.keys()
        {
            keys.push(k);
            //println!("pushed key: {}", k)
        }
    }

    let stack_consumer: &mut Vec<Item> = &mut Vec::new();
    let stack_finished_methods: &mut Vec<i64> = &mut Vec::new();
    let stack_failed: &mut Vec<Item> = &mut Vec::new();


    if !map_methods.is_empty()
    {
        let mut it = 0;
        let end = map_methods.len() as i64;

        if *count_iterated == 0
        {
            *reset_it_start = false;
        }
        else if it != end
        {
            *it_start += 1;
            it = *it_start;
        }
        else
        {
            // ???
            // nothing, maybe?
        }

        while it < end
        {
            /*if *method_count % 5000 == 0
            {
                println!("Method Count = {}", method_count);
            }*/
            *method_count += 1;

            *it_start = it;
            *reset_it_start = false;
            *count_iterated += 1;

            //println!("keys[it as usize] is {}", keys[it as usize]);
            //println!("map_methods[keys[it as usize] ].item_key is {}", map_methods[keys[it as usize] ].item_key);

            let array_bounds: i64 = vector_items.len() as i64;

            // we added this additional condition to ensure that we don't go out of bounds when using item_key as an array index.
            if (map_methods[keys[it as usize] ].item_key >= 0) && (map_methods[keys[it as usize] ].item_key < array_bounds)
            {
                if vector_items[map_methods[keys[it as usize] ].item_key as usize].key == INT_MAX
                {
                    let mut item = Item::newFromKey(map_methods[keys[it as usize] ].item_key);
                    item.producer = *keys[(end - 1) as usize];
                    vector_items[map_methods[keys[it as usize] ].item_key as usize] = item;
                }
                else if vector_items[(TEST_SIZE * NUM_THRDS) as usize].key == INT_MAX
                {
                    let mut item = Item::newFromKey(map_methods[keys[it as usize] ].item_key);
                    item.producer = *keys[(end - 1) as usize];
                    vector_items[(TEST_SIZE * NUM_THRDS) as usize] = item;
                }
            }

            let mut vec_item;

            if (map_methods[keys[it as usize] ].item_key >= 0) && (map_methods[keys[it as usize] ].item_key < array_bounds)
            {
                vec_item = vector_items[map_methods[keys[it as usize] ].item_key as usize].clone();
            }
            else
            {
                vec_item = vector_items[(TEST_SIZE * NUM_THRDS) as usize].clone();
            }


            if map_methods[keys[it as usize] ].MethodType == MethodType::PRODUCER
            {
                account_balances.insert(map_methods[keys[it as usize]].item_key, map_methods[keys[it as usize]].item_val);

                vector_items[map_methods[keys[it as usize] ].item_key as usize].producer = *keys[it as usize]; // TODO: check that this line is reflected in all locations

                if vec_item.status == Status::ABSENT
                {
                    vector_items[map_methods[keys[it as usize] ].item_key as usize].status = Status::PRESENT;
                    vector_items[map_methods[keys[it as usize] ].item_key as usize].demote_methods.clear();
                }

                vector_items[map_methods[keys[it as usize] ].item_key as usize].add_int(1);

                if map_methods[keys[it as usize] ].semantics == Semantics::FIFO || map_methods[keys[it as usize] ].semantics == Semantics::LIFO
                {
                    for it_0 in map_methods.iter()
                    {
                        // is this the proper way to compare the iterators?
                        if it_0.1 == &map_methods[keys[it as usize] ] // TODO: should this be *it_0.1 == map_methods[keys[it as usize] ] ???
                        {
                            break;
                        }

                        if it_0.1.response < map_methods[keys[it as usize] ].invocation
                        {
                            let mut vec_item_0;
                            if it_0.1.item_key != INT_MIN
                            {
                                vec_item_0 = vector_items[it_0.1.item_key as usize].clone();
                            }
                            else
                            {
                                vec_item_0 = vector_items[(TEST_SIZE * NUM_THRDS) as usize].clone();
                            }



                            if (it_0.1.MethodType == MethodType::PRODUCER) && (vec_item_0.status == Status::PRESENT) && (map_methods[keys[it as usize] ].MethodType == MethodType::PRODUCER) && (it_0.1.semantics == Semantics::FIFO)
                            {
                                vec_item_0.promote_items.push(vec_item.key);
                                vector_items[map_methods[keys[it as usize] ].item_key as usize].demote();
                                vector_items[map_methods[keys[it as usize] ].item_key as usize].demote_methods.push(*it_0.1);
                            }

                            // NOTE: this was an if in the previous code, but they're mutually exclusive.
                            else if (it_0.1.MethodType == MethodType::PRODUCER) && (vec_item_0.status == Status::PRESENT) && (map_methods[keys[it as usize] ].MethodType == MethodType::PRODUCER) && (it_0.1.semantics == Semantics::LIFO)
                            {
                                vector_items[map_methods[keys[it as usize] ].item_key as usize].promote_items.push(vec_item_0.key);
                                vec_item_0.demote();
                                vec_item_0.demote_methods.push(map_methods[keys[it as usize] ]);
                            }
                        }
                    }
                }


            }
            else if map_methods[keys[it as usize] ].MethodType == MethodType::READER ||  map_methods[keys[it as usize] ].MethodType == MethodType::WRITER
            {
                let mut new_balance = 0;

                { // using the rust scope trick to create multiple borrows
                    let old_balance = account_balances.get(&map_methods[keys[it as usize]].item_key);
                    let is_sender;

                    // a negative account change means that this is the sender account.
                    if map_methods[keys[it as usize]].item_val < 0
                    {
                        is_sender = true;
                    }
                    else
                    {
                        is_sender = false;
                    }


                    if is_sender
                    {
                        if old_balance == None
                        {
                            println!("Sender account does not exist.");
                            return false;
                        }

                        if *old_balance.unwrap() < (-1 * map_methods[keys[it as usize]].item_val)
                        {
                            println!("Sender account has insufficient funds.");
                            println!("Account {} is attempting to send {}, but only has {}", map_methods[keys[it as usize]].item_key, (-1 * map_methods[keys[it as usize]].item_val), *old_balance.unwrap());
                            return false;
                        }
                    }
                    else // is receiver
                    {
                        // not really any restrictions on the receiver, or at least none that we can check at this point.
                    }

                    new_balance = old_balance.unwrap() + map_methods[keys[it as usize]].item_val;

                }
                account_balances.insert(map_methods[keys[it as usize]].item_key, new_balance);

                if map_methods[keys[it as usize] ].status == true
                {
                    vec_item.demote_r();

                    // also demote vec_item's assigner
                    if (map_methods[keys[it as usize] ].item_key >= 0) && (map_methods[keys[it as usize] ].item_key < array_bounds)
                    {
                        vector_items[map_methods[keys[it as usize] ].item_key as usize].demote_r();
                    }
                    else
                    {
                        vector_items[(TEST_SIZE * NUM_THRDS) as usize].demote_r();
                    }
                }
                else
                {
                    // TODO: do we need to call handle_failed_read on vec_item's assigner?
                    handle_failed_read(map_methods, vector_items, it, &mut vec_item, stack_failed);
                }
            }
            else if map_methods[keys[it as usize] ].MethodType == MethodType::CONSUMER
            {
                if map_methods[keys[it as usize] ].status == true
                {
                    if vec_item.sum > 0.0
                    {
                        vector_items[map_methods[keys[it as usize] ].item_key as usize].sum_r = 0.0;
                    }
                    vec_item.sub_int(1);
                    vec_item.status = Status::ABSENT;

                    // also modify vec_item's assigner
                    if (map_methods[keys[it as usize] ].item_key >= 0) && (map_methods[keys[it as usize] ].item_key < array_bounds)
                    {
                        vector_items[map_methods[keys[it as usize] ].item_key as usize].sub_int(1);
                        vector_items[map_methods[keys[it as usize] ].item_key as usize].status = Status::ABSENT;
                    }
                    else
                    {
                        vector_items[(TEST_SIZE * NUM_THRDS) as usize].sub_int(1);
                        vector_items[(TEST_SIZE * NUM_THRDS) as usize].status = Status::ABSENT;
                    }

                    if vec_item.sum < 0.0
                    {
                        let mut it_method = 0;
                        while it_method != vec_item.demote_methods.len()
                        {
                            if (map_methods[keys[it as usize] ].response < vec_item.demote_methods[it_method].invocation) || (vec_item.demote_methods[it_method].response < map_methods[keys[it as usize] ].invocation)
                            {
                            }
                            else
                            {
                                vec_item.promote();

                                // also promote vec_item's assigner
                                if (map_methods[keys[it as usize] ].item_key >= 0) && (map_methods[keys[it as usize] ].item_key < array_bounds)
                                {
                                    vector_items[map_methods[keys[it as usize] ].item_key as usize].promote();
                                }
                                else
                                {
                                    vector_items[(TEST_SIZE * NUM_THRDS) as usize].promote();
                                }

                                let mut vec_method_item;
                                if vec_item.demote_methods[it_method].item_key != INT_MIN
                                {
                                    vec_method_item = vector_items[vec_item.demote_methods[it_method as usize].item_key as usize].clone();
                                }
                                else
                                {
                                    vec_method_item= vector_items[(TEST_SIZE*NUM_THRDS) as usize].clone();
                                }


                                let mut tempStack = Vec::new();

                                while !vec_method_item.promote_items.is_empty()
                                {
                                    let top = vec_method_item.promote_items[vec_method_item.promote_items.len() - 1].clone();
                                    if top != map_methods[keys[it as usize] ].item_key
                                    {
                                        tempStack.push(top);
                                    }

                                    vec_method_item.promote_items.pop();

                                    // also pop promote_items for vec_method_item's assigner
                                    if vec_item.demote_methods[it_method].item_key != INT_MIN
                                    {
                                        vector_items[vec_item.demote_methods[it_method as usize].item_key as usize].promote_items.pop();
                                    }
                                    else
                                    {
                                        vector_items[(TEST_SIZE*NUM_THRDS) as usize].promote_items.pop();
                                    }
                                }

                                //Swapping the two stacks
                                let s = vec_method_item.promote_items.clone();
                                vec_method_item.promote_items = tempStack.clone();


                                vec_method_item.demote_methods.remove(it_method);


                                // remove it_method from the demote_methods of vec_method_item's assigner, too.
                                // also swap the stack with the assigner.
                                if vec_item.demote_methods[it_method].item_key != INT_MIN
                                {
                                    vector_items[vec_item.demote_methods[it_method as usize].item_key as usize].demote_methods.remove(it_method);
                                    vector_items[vec_item.demote_methods[it_method as usize].item_key as usize].promote_items = tempStack.clone();
                                }
                                else
                                {
                                    vector_items[(TEST_SIZE*NUM_THRDS) as usize].demote_methods.remove(it_method);
                                    vector_items[vec_item.demote_methods[it_method as usize].item_key as usize].promote_items = tempStack.clone();
                                }

                                tempStack = s;

                                it_method -= 1;

                            }

                            it_method += 1;
                        }

                    }

                    stack_consumer.push(vec_item.clone());
                    stack_finished_methods.push(*keys[it as usize]);


                    if vec_item.producer != *keys[(end - 1) as usize]
                    {
                        stack_finished_methods.push(vec_item.producer);
                    }

                }
                else
                {
                    handle_failed_consumer(map_methods,vector_items,it, &mut vec_item,stack_failed)
                }
            }

            it += 1;
        }

        if *reset_it_start == true
        {
            *it_start -= 1;
        }

        while !stack_consumer.is_empty()
        {
            let it_top = stack_consumer[stack_consumer.len() - 1].clone();
            let mut item_promote: i64;
            while !it_top.promote_items.is_empty()
            {
                let stack_consumer_last = stack_consumer.len() - 1;
                let promote_items_last = it_top.promote_items.len() - 1;
                item_promote = stack_consumer[stack_consumer_last].promote_items[promote_items_last];
                if item_promote != INT_MIN
                {
                    vector_items[item_promote as usize].promote();
                }
                else
                {
                    vector_items[(TEST_SIZE*NUM_THRDS) as usize].promote();
                }
                let last = stack_consumer.len() - 1;
                stack_consumer[last].promote_items.pop();
            }

            stack_consumer.pop();
        }

        while !stack_failed.is_empty()
        {
            let sfLast = stack_failed.len() - 1;

            if stack_failed[sfLast].status == Status::PRESENT
            {
                stack_failed[sfLast].demote_f();
            }

            stack_failed.pop();
        }


        while !stack_finished_methods.is_empty()
        {
            let sFTop = stack_finished_methods.len() - 1;
            let mut method_top = stack_finished_methods[sFTop];

            //println!("keys holds {}", keys[*it_start as usize]);
            //println!("map_methods size is {}", map_methods.len());

            // we added this condition to account for no NULL in rust
            if *keys[*it_start as usize] < map_methods.len() as i64
            {
                if map_methods[&method_top].item_key != map_methods[keys[*it_start as usize]].item_key
                {
                    map_methods.remove(&method_top);

                    // TODO: originally these two lines were switched (the other one was commented-out)
                    // ASK CHRIS ABOUT THIS LATER
                    //method_top -= 1;
                    stack_finished_methods[sFTop] -= 1;
                }
            }


            stack_finished_methods.pop();

        }

        let mut outcome = true;
        let mut vec_verify: Item;

        for i in 0..(TEST_SIZE*NUM_THRDS+1)
        {
            vec_verify = vector_items[i as usize].clone();

            if vec_verify.key == INT_MAX // vec_verify == NULL
            {
                continue;
            }

            if vec_verify.sum < 0.0
            {
                outcome = false;
                println!("WARNING: Item {}, Incorrect CONSUME, sum {}\n", vec_verify.key, vec_verify.sum);
            }
            else if (vec_verify.sum.ceil() + vec_verify.sum_r) < 0.0
            {
                outcome = false;
                println!("WARNING: Item {}, Incorrect READ/WRITE, sum_r {}\n", vec_verify.key, vec_verify.sum_r);
            }








            /*let N: i64;

            if vec_verify.sum_f == 0.0
            {
                N = 0;
            }
            else
            {
                N = -1;
            }

            if ((vec_verify.sum.ceil() + vec_verify.sum_f) * N as f32) < 0
            {
                outcome = false;

                println!("WARNING: Item {}, Incorrect FAIL, sum_f {}\n", vec_verify.key, vec_verify.sum_f);
            }*/

            // TODO: also get the final account (map) configuration from hyperledger fabric, and compare it with ours

        }

        /*if outcome == true
        {
            println!("-------------Program Correct Up To This Point-------------\n");
        }
        else
        {
            println!("-------------Program Not Correct-------------\n");
        }*/


        final_outcome = outcome;
    }
    return final_outcome;
}
/*
TODO: make sure that when removing "it" from map methods, that we properly handle the discrepancy of array position vs key value.

*/


/*

fn get_process_id(thread_map: Arc<CHashMap<thread::ThreadId,i64>>, thread_ctr: Arc<AtomicI64>) -> i64
{
    let this_id = thread::current().id();
    let _process: i64;
    if !thread_map.contains_key(&this_id)
    {
        _process = thread_ctr.fetch_add(1, Ordering::SeqCst);
        thread_map.insert_new(this_id, _process);
    }
    else
    {
        _process = *thread_map.get(&this_id).unwrap();
    }

    return _process;
}


fn get_method_id(thread_map: Arc<CHashMap<thread::ThreadId,i64>>,thread_ctr: Arc<AtomicI64>,method_id: Arc<Mutex<Vec<i64>>>) -> i64
{
    let _process = get_process_id(thread_map.clone(),thread_ctr.clone());
    let mut method_id_lock = method_id.lock().unwrap();
    if method_id_lock[_process as usize] == 0
    {
        method_id_lock[_process as usize] = _process + 1;
    }
    else
    {
        method_id_lock[_process as usize] = method_id_lock[_process as usize] + NUM_THRDS as i64;
    }

    return method_id_lock[_process as usize];

}


fn create_method(_item_key: i64, _item_val: i64, _semantics: Semantics, _type: MethodType ,_invocation: i64, _response: i64, _status: bool, thread_map: Arc<CHashMap<thread::ThreadId,i64>>,thread_ctr: Arc<AtomicI64>, method_id: Arc<Mutex<Vec<i64>>>, thrd_lists: Arc<Mutex<Vec<LinkedList<Method>>>>)
{

    let _process: i64 = get_process_id(thread_map.clone(), thread_ctr.clone());
    let m_id: i64 = get_method_id(thread_map.clone(), thread_ctr.clone(),method_id.clone());
    //let empty: bool = thrd_lists[_process].is_empty();

    let m1 = Method::Method(m_id, _process, _item_key, _item_val, _semantics, _type, _invocation, _response, _status);

    {   // acquire the lock on the method list so we can add this method to it
        let mut thrd_list = thrd_lists.lock().unwrap();
        thrd_list[_process as usize].push_back(m1);
    }   // the lock is released at the end of this scope we created

    //if empty
    {
        //thrd_lists_itr[_process] = thrd_lists[_process].begin();
    }

    //update_method_time(_invocation, _response);

}*/

fn verify_block(account_balances: &mut HashMap<i64, i64>, block: &mut LinkedList<Transaction>, all_methods: &mut LinkedList<Method>, expected_balances: &mut HashMap<i64, i64>) -> bool
{
    println!("--Verifying new block--");
    //println!("Initial account balances:");

    //println!("{:?}", account_balances);

    //println!("Test size = {}", TEST_SIZE);
    //println!("Number of Threads = {}", NUM_TOTAL_THRDS);
    // rust doesn't like global variables, so Arc is used to allow the object to be shared and modified by multiple threads.
    // CHashMap is a concurrent hash map. normal rust hashmaps don't support concurrency.

    //let map_clone = map.clone(); // rust moves ownership of map when creating threads, so we need to keep a copy in this scope for later access

    // this barrier is used to ensure that all map worker threads, as well as the verifier thread, start their jobs at the same time
    //let barrier = Arc::new(Barrier::new(NUM_THRDS as usize));

    // this array stores the status of the worker threads for the verifier thread to check

    let thread_map = Arc::new(CHashMap::<thread::ThreadId,i64>::new());
    let thread_ctr = Arc::new(AtomicI64::new(0));
    let method_id = Arc::new(Mutex::new(vec![0;NUM_THRDS as usize]));

    let method_count = &mut 0;
    //let mut vector_itmes = Vec!()



    // TODO: consider using vectors instead of linked-lists ("usually faster in rust")



    //let start = Instant::now();
    //let mut threads = Vec::with_capacity(NUM_TOTAL_THRDS as usize);
    //let epoch = Instant::now();





/*
        HERE is where we will:
        1. create some simulated transactions (in a vector)
        2. populate the hashmap with the initial balances of each account involved in transactions (using producer operations)

        THEN the function we have each thread call will:
        1. execute their own set of transactions (and create the equivalent vector space Methods accordingly)

        LASTLY, the verifier thread (which will execute afterwards - that is, sequentially) will:
        1. do as it currently does, but also check for additional things, like negative balances, and double-spending. (maybe also, we should calculate a sum of all balances before and after. they should be equal, because transactions should only ever move amounts, not create or delete existing.)
        2. can remove logic for consumer, because we will not delete accounts.
*/

    // create a map of account balances, and fill it with initial balances (before any transactions occur)
    // TODO: make this the concurrent map, which will be used by the threads.


    // all_methods will get passed to the verifier, where it will simulate the actions on the account_balances map, based on the timings.
    // TODO: we need to encode all transaction info into the methods.
    // NOTE: we also need to create 2 methods for each transaction: one that removes an amount from the sender's balance, and one that adds to the receivers.





    /*for reference:

    struct Transaction
    {
        amount: i64,
        new_sender_balance: u64,
        new_receiver_balance: u64,
        send_addr: i64, // NOTE: I think for actual blockchains, wallets have separate sender and receiver addresses, but for our simulation, we'll just treat them as if they are the same.
        receive_addr: i64
    }
*/


    // below are the hyperledger-specific correctness conditions:
    let mut invalid_txn = false;
    for txn in block.iter()
    {
        println!("\n-New transaction-\nSender address: {}\nReceiver address: {}\nTransaction amount: {}\n ", txn.send_addr, txn.receive_addr, txn.amount);
        if txn.send_addr == txn.receive_addr
        {
            println!("Transaction attempts to send an amount from an account to itself.");
            invalid_txn = true;
            break;
        }

        if txn.amount < 0
        {
            println!("Transaction attempts to send a negative amount.");
            invalid_txn = true;
            break;
        }


        // creates a producer method for a receiver account that didn't exist prior to this transaction
        // NOTE: hyperledger fabric creates accounts for receiving addresses, if they do not exist yet.
        // source: line 459 of https://github.com/hyperledger/fabric-samples/blob/5e933c10cbddfceb9b544c6a5cefbfe101594548/token-erc-20/chaincode-go/chaincode/token_contract.go
        let mut has_producer = false;
        for m in all_methods.clone()
        {
            if (m.MethodType == MethodType::PRODUCER) && (m.item_key == txn.receive_addr)
            {
                has_producer = true;
                break;
            }
        }
        if has_producer == false
        {
            all_methods.push_back(Method::Method(0, 0, txn.receive_addr, 0 as i64, Semantics::MAP, MethodType::PRODUCER, 0, 0, true));
        }

        all_methods.push_back(Method::Method(0, 0, txn.send_addr, -1 * txn.amount, Semantics::MAP, MethodType::WRITER, 0, 0, true));
        all_methods.push_back(Method::Method(0, 0, txn.receive_addr, txn.amount, Semantics::MAP, MethodType::WRITER, 0, 0, true));





/*
        { // using the rust scope trick to create multiple borrows
            let old_sender_balance = account_balances.get(&txn.send_addr).unwrap();
            if *account_balances.get(&txn.send_addr).unwrap() != txn.new_sender_balance as i64
            {
                println!("Failed transaction: sender's new account balance is not equal to what the transaction expected.");
                invalid_txn = true;
                break;
            }
        }
        {
            let old_receiver_balance = account_balances.get(&txn.receive_addr).unwrap();
            if *account_balances.get(&txn.receive_addr).unwrap() != txn.new_receiver_balance as i64
            {
                println!("Failed transaction: receiver's new account balance is not equal to what the transaction expected.");
                invalid_txn = true;
                break;
            }
        }
*/
        //account_balances.insert(txn.send_addr, txn.new_sender_balance as i64);
        //account_balances.insert(txn.receive_addr, txn.new_receiver_balance as i64);
    }




    // for each of these insertions, we also need to create corresponding Method objects

    // create dummy transactions for the simulation, and fill a vector with them
    // will have 5 transactions, with 3 accounts (a, b, c). all 3 accounts start with balance of 10.
    let mut result = !invalid_txn;

    if invalid_txn == false
    {
        result = verify(account_balances, all_methods, method_count);
    }

    //println!("Verified: {}", result);




    if result == false
    {
        return false;
    }
    else
    {
        let hlf_accounts: HashSet<i64> = account_balances.keys().cloned().collect();
        let tool_accounts: HashSet<i64> = expected_balances.keys().cloned().collect();
        if hlf_accounts != tool_accounts
        {
            return false
        }
        else
        {
            for k in hlf_accounts.iter()
            {
                if account_balances.get(k) == None || expected_balances.get(k) == None
                {
                    return false
                }
                else
                {
                    if account_balances.get(k).unwrap() != expected_balances.get(k).unwrap()
                    {
                        println!("Final account balances:");
                        println!("{:?}\n\n", account_balances);
                        println!("Expected account balances:");
                        println!("{:?}\n\n", expected_balances);
                        return false
                    }
                }
            }
        }


        println!("Final account balances:");
        println!("{:?}\n\n", account_balances);
    }


    return result
}
/*
#[repr(C)]
pub struct transaction
{
    // c's char is 1 byte, but rust's char is 4 bytes.
    // if we just read this value in rust, it will give us the numeric representation (97 instead of 'a')
    // so we need to typecast libc::c_char to u8, then typecase to char.
    pub sender: libc::c_char,
    pub receiver: libc::c_char,
    pub amount: libc::c_int
}*/
use std::ffi::CStr;
#[no_mangle]
pub extern "C" fn go_rust_connector(before_state: *const c_char, transactions: *const c_char, after_state: *const c_char) -> i32
//pub extern "C" fn test(s: transaction) -> i32
{
    let mut txn_list = LinkedList::<Transaction>::new();

    // converting the transaction information from c strings to rust strings.
    let c_str: &CStr = unsafe { CStr::from_ptr(transactions) };
    let str_slice: &str = c_str.to_str().unwrap();
    // split str_slice by |
    // iterate over the splitted strings, converting them back into events.
    let mut split = str_slice.split('|');

    let mut counter = 0;
    let mut address_unique_ids = HashMap::<String, i64>::new();

    for txn_str in split
    {
        // decodes the string into an event object
        let e: event = serde_json::from_str(txn_str).unwrap();
        println!("trans is {} {} {}", e.to, e.from, e.value);

        // generates unique ids for the addresses (so we don't have to deal with base 64 strings all the time)
        if address_unique_ids.get(&e.to) == None
        {
            address_unique_ids.insert(e.to.clone(), counter);
            counter += 1;
        }

        if address_unique_ids.get(&e.from) == None
        {
            address_unique_ids.insert(e.from.clone(), counter);
            counter += 1;
        }

        let receiver_id = *address_unique_ids.get(&e.to).unwrap();
        let sender_id = *address_unique_ids.get(&e.from).unwrap();

        // creates a transaction object from the decoded event struct,
        // and adds it to our master list of transactions
        let t = Transaction::from_event(e, sender_id, receiver_id);
        txn_list.push_back(t);
    }

    // TODO: anywhere that addresses are used, we'll need to use the map of unique addresses.





    let mut all_methods = LinkedList::<Method>::new();
    let mut account_balances = HashMap::<i64, i64>::new();

    // creates producer methods for all accounts, based on the state before the block.
    //the state will look like this:
    //"address1_in_b64":account_balance_as_i64|"address2_in_b64":account_balance_as_i64
    // converting the transaction information from c strings to rust strings.
    let c_str: &CStr = unsafe { CStr::from_ptr(before_state) };
    let str_slice: &str = c_str.to_str().unwrap();
    // split str_slice by |
    // iterate over the splitted strings, converting them back into events.
    let mut split = str_slice.split('|');

    for address_and_balance in split
    {
        // .split() returns an iterator.
        let mut single_account = address_and_balance.split(':');
        let address = single_account.next().unwrap();
        let balance = single_account.next().unwrap().parse();

        let id = address_unique_ids.get(address);
        if id == None
        {
            println!("Unknown account in before-state. This account is not involved in any transactions for this block.");
            return -1;
        }
        else
        {

            let producer = Method::Method(0, 0, *(id.unwrap()), balance.unwrap(), Semantics::MAP, MethodType::PRODUCER, 0, 0, true);
            all_methods.push_back(producer);
        }
    }

    let mut expected_balances = HashMap::<i64, i64>::new();

    let c_str: &CStr = unsafe { CStr::from_ptr(after_state) };
    let str_slice: &str = c_str.to_str().unwrap();
    // split str_slice by |
    // iterate over the splitted strings, converting them back into events.
    let mut split = str_slice.split('|');
    for address_and_balance in split
    {
        // .split() returns an iterator.
        let mut single_account = address_and_balance.split(':');
        let address = single_account.next().unwrap();
        let balance = single_account.next().unwrap().parse();

        let id = address_unique_ids.get(address);
        if id == None
        {
            println!("Unknown account in after-state. This account is not involved in any transactions for this block.");
            return -1;
        }
        else
        {
            expected_balances.insert(*(id.unwrap()), balance.unwrap());
        }
    }

    let result = verify_block(&mut account_balances, &mut txn_list, &mut all_methods, &mut expected_balances) as i32;

    // TODO: take the state after the block, and compare against what's in our account_balances map. return false if they don't match (because there was a double-spend in the concurrent execution)
    // NOTE: account_balances will be updated by this point, and we can safely use it here without rust being upset.

    return result

/*
DEMO:
    // verify the first block, with 5 transactions and 3 accounts below:
    {
        let mut all_methods = LinkedList::<Method>::new();
        let mut account_balances = HashMap::<i64, i64>::new();

        // these producer methods create the three accounts involved in the transactions, each with balances of 10.
        let m1 = Method::Method(0, 0, 1, 10, Semantics::MAP, MethodType::PRODUCER, 0, 0, true);
        let m2 = Method::Method(0, 0, 2, 10, Semantics::MAP, MethodType::PRODUCER, 0, 0, true);
        let m3 = Method::Method(0, 0, 3, 10, Semantics::MAP, MethodType::PRODUCER, 0, 0, true);
        account_balances.insert(1, 10);
        account_balances.insert(2, 10);
        account_balances.insert(3, 10);

        all_methods.push_back(m1);
        all_methods.push_back(m2);
        all_methods.push_back(m3);

        // NOTE: these are dummy transactions for testing purposes, but we will need to get these transactions from hyperledger fabric later.
        // TODO: we will also need to get the timing info from hyperledger fabric, too.
        // TODO: lastly, we need to get hyperledger's account balances to compare against ours.
        let mut block = LinkedList::<Transaction>::new();
        // starting balances: 10, 10, 10
        let txn1 = Transaction {amount: 5, send_addr: 1, receive_addr: 2};
        let txn2 = Transaction {amount: 5, send_addr: 1, receive_addr: 3};
        let txn3 = Transaction {amount: 10, send_addr: 2, receive_addr: 1};
        let txn4 = Transaction {amount: 10, send_addr: 3, receive_addr: 1};
        let txn5 = Transaction {amount: 5, send_addr: 1, receive_addr: 2};
        // ending balances: 5, 15, 10
        block.push_back(txn1);
        block.push_back(txn2);
        block.push_back(txn3);
        block.push_back(txn4);
        block.push_back(txn5);

        let result = verify_block(&mut account_balances, &mut block, &mut all_methods) as i32;

        return result
    }
*/

}
