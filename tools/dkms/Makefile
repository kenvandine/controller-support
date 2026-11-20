# Out-of-tree build of hid-xbox plus the matching (Xbox-free) hid-microsoft,
# so the pair can be tested on a stock Ubuntu kernel without rebuilding it.
obj-m += hid-xbox.o hid-microsoft.o

KVER ?= $(shell uname -r)
KDIR ?= /lib/modules/$(KVER)/build

all:
	$(MAKE) -C $(KDIR) M=$(CURDIR) modules

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) clean
