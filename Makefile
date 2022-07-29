DEBUG := false
OUT_DIR := out
KEY_DIR := keys
SRC_DIR := src
TARGET := local
CACHE_DIR := cache
CONFIG_DIR := config
TOOLCHAIN_DIR := toolchain
SRC_DIR := src
USER := $(shell id -g):$(shell id -g)
CPUS := $(shell nproc)
ARCH := x86_64

include $(PWD)/config/global.env
include $(PWD)/make/keys.mk
include $(PWD)/make/fetch.mk
include $(PWD)/toolchain/Makefile

.DEFAULT_GOAL := default
.PHONY: default
default: fetch $(OUT_DIR)/bzImage

# Clean repo back to initial clone state
.PHONY: clean
clean:
	rm -rf cache out
	docker image rm -f local/$(NAME)-build

# Launch a shell inside the toolchain container
.PHONY: toolchain-shell
toolchain-shell: $(OUT_DIR)/toolchain.tar
	$(call toolchain,root,bash)

# Pin all packages in toolchain container to latest versions
.PHONY: toolchain-update
toolchain-update:
	$(call toolchain,root,toolchain-update )

# Source anything required from the internet to build
.PHONY: fetch
fetch: \
	keys \
	$(OUT_DIR) \
	$(OUT_DIR)/toolchain.tar \
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
	rm $(CONFIG_DIR)/$(TARGET)/linux.config
	make $(CONFIG_DIR)/$(TARGET)/linux.config

# This can likely be eliminated with path fixes in toolchain/Makefile
$(OUT_DIR)/toolchain.tar:
	ARCH=$(ARCH) \
	OUT_DIR=../$(OUT_DIR) \
	DEBIAN_HASH=$(DEBIAN_HASH) \
	$(MAKE) -C $(TOOLCHAIN_DIR) \
	../$(OUT_DIR)/toolchain.tar

$(CONFIG_DIR)/busybox.config:
	$(call toolchain,$(USER), " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		KCONFIG_NOTIMESTAMP=1 make menuconfig && \
		cp .config /config/busybox.config; \
	")

$(CONFIG_DIR)/$(TARGET)/linux.config:
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		make menuconfig && \
		cp .config /config/$(TARGET)/linux.config; \
	")

$(OUT_DIR)/busybox: \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION) \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2 \
	$(CACHE_DIR)/busybox-$(BUSYBOX_VERSION).tar.bz2.sig
	$(call toolchain,$(USER)," \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		cp /config/busybox.config .config && \
		make -j$(CPUS) busybox && \
		cp busybox /out/; \
	")

$(OUT_DIR)/sample_init:
	$(call toolchain,$(USER)," \
		gcc \
			-static \
			-static-libgcc /src/sample/init.c \
			-o /out/sample_init; \
	")

$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio: \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.xz \
	$(CACHE_DIR)/linux-$(LINUX_VERSION).tar.sign
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		gcc usr/gen_init_cpio.c -o usr/gen_init_cpio \
	")

$(OUT_DIR)/rootfs.cpio: \
	$(OUT_DIR)/busybox \
	$(OUT_DIR)/sample_init \
	$(CACHE_DIR)/linux-$(LINUX_VERSION)/usr/gen_init_cpio
	mkdir -p $(CACHE_DIR)/rootfs/bin
ifeq ($(DEBUG), true)
	cp $(OUT_DIR)/sample_init $(CACHE_DIR)/rootfs/sample_init
	cp $(SRC_DIR)/scripts/busybox_init $(CACHE_DIR)/rootfs/init
	cp $(OUT_DIR)/busybox $(CACHE_DIR)/rootfs/bin/
else
	cp $(OUT_DIR)/sample_init $(CACHE_DIR)/rootfs/init
endif
	$(call toolchain,$(USER)," \
		cd /cache/rootfs && \
		find . -mindepth 1 -execdir touch -hcd "@0" "{}" + && \
		find . -mindepth 1 -printf '%P\0' && \
		cd /cache/linux-$(LINUX_VERSION) && \
		usr/gen_initramfs.sh \
			-o /out/rootfs.cpio \
			/config/$(TARGET)/rootfs.list && \
		cpio -itv < /out/rootfs.cpio && \
		sha256sum /out/rootfs.cpio; \
	")

$(OUT_DIR)/bzImage: \
	$(OUT_DIR)/rootfs.cpio
	$(call toolchain,$(USER)," \
		cd /cache/linux-$(LINUX_VERSION) && \
		cp /config/$(TARGET)/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		cp arch/x86_64/boot/bzImage /out/ && \
		sha256sum /out/bzImage; \
	")
