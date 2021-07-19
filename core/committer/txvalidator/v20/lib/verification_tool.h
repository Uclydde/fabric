// we need free() so that the go code can free the C strings it creates.
#include <stdlib.h>

//int test(char s[]);
/*typedef struct {
    char* sender;
    char* receiver;
    int amount;
} transaction;*/

//int test(transaction s);
int go_rust_connector(char* before_state, char* transactions);//, char* after_state);
