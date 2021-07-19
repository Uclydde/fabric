package txvalidator

import "fmt"
//import "encoding/json"
import "os"
import "unsafe"
import "container/list"

/*
#cgo LDFLAGS: -L./lib -lverify
#include "./lib/verification_tool.h"
*/
import "C"

import "encoding/json"



/*func main() {
  // the transactions are stored as a list of events
  txn_events := list.New()

  test_transaction1 := event {
    From: "Christina",
    To:  "Damian",
    Value: 1,
  }

  test_transaction2 := event {
    From: "Julian",
    To:  "Chris",
    Value: 5,
  }

  data, err := json.Marshal(test_transaction1)
  if err != nil {
      fmt.Fprintln(os.Stderr, err)
  }
  txn_str1 := string(data)

  data, err = json.Marshal(test_transaction2)
  if err != nil {
      fmt.Fprintln(os.Stderr, err)
  }
  txn_str2 := string(data)
  fmt.Print(txn_str2)

  txn_events.PushBack(txn_str1)
  txn_events.PushBack(txn_str2)


  //result := check_correctness("TEST STATE1", string(data), "TEST STATE2") // TODO: will need to marshall each event in the list, and cast the data as a string.
  result := check_correctness("Julian:10|Chris:10|Christina:10|Damian:10", txn_events, "Julian:5|Chris:15|Christina:9|Damian:11")
  fmt.Print("Transactions correct? ", result, "\n");
}*/

type event struct {
	From  string `json:"from"`
	To    string `json:"to"`
	Value int    `json:"value"`
}

/*
func main() {
	// the transactions are stored as a list of events
	txn_events := list.New()

  test_transaction1 := event {
    From: "Christina",
    To:  "Damian",
    Value: 1,
  }

	test_transaction2 := event {
		From: "Julian",
		To:  "Chris",
		Value: 5,
	}

	data, err := json.Marshal(test_transaction1)
	if err != nil {
			fmt.Fprintln(os.Stderr, err)
	}
	txn_str1 := string(data)

	data, err = json.Marshal(test_transaction2)
	if err != nil {
			fmt.Fprintln(os.Stderr, err)
	}
	txn_str2 := string(data)
	fmt.Print(txn_str2)

	txn_events.PushBack(txn_str1)
	txn_events.PushBack(txn_str2)


	//result := check_correctness("TEST STATE1", string(data), "TEST STATE2") // TODO: will need to marshall each event in the list, and cast the data as a string.
	result := check_correctness("Julian:10|Chris:10|Christina:10|Damian:10", txn_events, "Julian:5|Chris:15|Christina:9|Damian:11")
	fmt.Print("Transactions correct? ", result, "\n");
}*/


// TODO: modify this wrapper function to take the states and transactions in the format that hyperledger maintains them.
// TO CALL THIS FUNCTION:
// the before_state is in the format "address:balance|address:balance". only the accounts involved in the transactions should be in this string. otherwise the correctness tool won't work.
// the transactions is a list of json strings.
// the after_state is just like before_state.
func check_correctness(before_state string, transactions *list.List, after_state string) bool {

	// iterates through the list of transactions, converting all of them from event objects to strings,
	// and appends all of the strings to one master string (with | separating them).
	txns_str := ""
	for txn := transactions.Front(); txn != nil; txn = txn.Next() {
		txn_str, err := txn.Value.(string)
		fmt.Print(txn_str)
		if err == false {
				fmt.Fprintln(os.Stderr, err)
		}

		if txns_str == "" {
			txns_str += txn_str
		} else {
			txns_str += "|" + txn_str
		}
	}

	// converts the input to the format needed by the correctness tool in rust (c strings)
	state1 := C.CString(before_state)
	txns := C.CString(txns_str)
	state2 := C.CString(after_state)

	// frees the C pointers' memory at the end of the function.
	defer C.free(unsafe.Pointer(state1))
	defer C.free(unsafe.Pointer(txns))
	defer C.free(unsafe.Pointer(state2))

	// passes the information (as C strings) to the correctness tool (in rust)
	fmt.Print("go is passing ", txns_str, "\n")
	result := C.go_rust_connector(state1, txns, state2)

	// go doesn't support typecasting ints as bools...
	if result == 1 {
		return true
	} else if result == 0 {
		return false
	} else {
		fmt.Print("Error in input to correctness tool.")
		return false
	}
}
