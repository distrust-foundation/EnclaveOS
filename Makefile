NAME := qos
DEBUG := true
OUT_DIR := out
KEY_DIR := keys
TARGET := local
CACHE_DIR := cache
CONFIG_DIR := targets/$(TARGET)
SCRIPTS_DIR := scripts
CPUS := $(shell nproc)
ARCH := x86_64

include $(PWD)/config.env
include $(PWD)/make/keys.mk
include $(PWD)/make/fetch.mk
include $(PWD)/make/extract.mk
include $(PWD)/make/toolchain.mk

.DEFAULT_GOAL := default
default: $(OUT_DIR)/bzImage

$(CONFIG_DIR)/busybox.config:
	$(toolchain) " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		KCONFIG_NOTIMESTAMP=1 make menuconfig && \
		cp .config /config/busybox.config; \
	"

$(CONFIG_DIR)/linux.config:
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		make menuconfig && \
		cp .config /config/linux.config; \
	"

$(OUT_DIR)/rootfs.cpio: $(OUT_DIR)/busybox
	mkdir -p $(CACHE_DIR)/rootfs/bin
ifdef DEBUG
	cp $(OUT_DIR)/busybox $(CACHE_DIR)/rootfs/bin;
	cp $(SCRIPTS_DIR)/busybox_init $(CACHE_DIR)/rootfs/init;
	chmod +x $(CACHE_DIR)/rootfs/init;
endif
	$(toolchain) " \
		cd /cache/rootfs \
		&& find . \
		| cpio -o -H newc \
		| gzip -f - > /out/rootfs.cpio \
	"

# Currently broken determinism attempt
#    $(toolchain) " \
#    	cd /cache/rootfs \
#    	&& mkdir -p dev \
#    	&& fakeroot mknod -m 0622 dev/console c 5 1 \
#    	&& find . -mindepth 1 -execdir touch -hcd "@0" "{}" + \
#    	&& find . -mindepth 1 -printf '%P\0' \
#    	| sort -z \
#    	| LANG=C bsdtar --uid 0 --gid 0 --null -cnf - -T - \
#    	| LANG=C bsdtar --null -cf - --format=newc @- \
#    " > $@


$(OUT_DIR)/busybox: extract
	$(toolchain) " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		cp /config/busybox.config .config && \
		make -j$(CPUS) busybox && \
		cp busybox /out/; \
	"

$(OUT_DIR)/bzImage: extract $(OUT_DIR)/rootfs.cpio
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		cp /config/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		cp arch/x86_64/boot/bzImage /out/; \
	"

.PHONY: run
run:
	qemu-system-x86_64 \
		-m 512M \
		-nographic \
		-initrd $(OUT_DIR)/rootfs.cpio \
		-kernel $(OUT_DIR)/bzImage
