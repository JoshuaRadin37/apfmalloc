#include <stdio.h>
#include "lrmalloc_rs.h"

int main(int argc, char* argv[]) {
    int* test = malloc(sizeof(int));
    *test = 3;
    
    
    
    free(test);
    
    unsigned char b = check_override();
    printf("Check Override output: %d\n", b);
    
	return !b;
}
