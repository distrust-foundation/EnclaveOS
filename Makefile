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
include $(PWD)/make/toolchain.mk

.DEFAULT_GOAL := default
.PHONY: default
default: fetch $(OUT_DIR)/bzImage

# Clean repo back to initial clone state
.PHONY: clean
clean:
	rm -rf cache out
	docker image rm -f local/$(NAME)-build

# Source anything required from the internet to build
.PHONY: fetch
fetch: \
	toolchain \
	keys \
	$(OUT_DIR) \
	$(CACHE_DIR) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig

# Build latest image and run in terminal via Qemu
.PHONY: run
run: default
	qemu-system-x86_64 \
		-m 512M \
		-nographic \
		-kernel $(OUT_DIR)/bzImage

# Run ncurses busybox config menu and save output
.PHONY: busybox-config
busybox-config:
	rm $(CONFIG_DIR)/busybox.config
	make $(CONFIG_DIR)/busybox.config

# Run linux config menu and save output
.PHONY: linux-config
linux-config:
	rm $(CONFIG_DIR)/linux.config
	make $(CONFIG_DIR)/linux.config

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

$(OUT_DIR)/busybox: \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION) \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig
	$(toolchain) " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		cp /config/busybox.config .config && \
		make -j$(CPUS) busybox && \
		cp busybox /out/; \
	"

$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio: \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		gcc usr/gen_init_cpio.c -o usr/gen_init_cpio \
	"

$(OUT_DIR)/rootfs.cpio: \
	$(OUT_DIR)/busybox \
	$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio
	mkdir -p $(CACHE_DIR)/rootfs/bin
	cp $(SCRIPTS_DIR)/busybox_init $(CACHE_DIR)/rootfs/init
	cp $(OUT_DIR)/busybox $(CACHE_DIR)/rootfs/bin/
	$(toolchain) " \
		cd /cache/rootfs && \
		find . -mindepth 1 -execdir touch -hcd "@0" "{}" + && \
		find . -mindepth 1 -printf '%P\0' && \
		cd /cache/linux-$(LINUX_VERSION) && \
		usr/gen_initramfs.sh -o /out/rootfs.cpio /config/rootfs.list && \
		cpio -itv < /out/rootfs.cpio && \
		sha256sum /out/rootfs.cpio; \
	"

$(OUT_DIR)/bzImage: \
	$(OUT_DIR)/rootfs.cpio
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		cp /config/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		cp arch/x86_64/boot/bzImage /out/ && \
		sha256sum /out/bzImage; \
	"
