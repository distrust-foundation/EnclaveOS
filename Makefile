NAME := qos
DEBUG := false
OUT_DIR := out
KEY_DIR := keys
TARGET := local
CACHE_DIR := cache
CONFIG_DIR := targets/$(TARGET)
CPUS := $(shell nproc)

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
		make menuconfig && \
		cp .config /config/busybox.config; \
	"

$(CONFIG_DIR)/linux.config:
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		make menuconfig && \
		cp .config /config/linux.config; \
	"
$(OUT_DIR)/busybox: extract
	$(toolchain) " \
		cd /cache/busybox-$(BUSYBOX_VERSION) && \
		cp /config/busybox.config .config && \
		make -j$(CPUS) busybox && \
		cp busybox /out/; \
	"

$(OUT_DIR)/bzImage: extract $(OUT_DIR)/busybox
	$(toolchain) " \
		cd /cache/linux-$(LINUX_VERSION) && \
		cp /config/linux.config .config && \
		make -j$(CPUS) bzImage && \
		cp arch/x86_64/boot/bzImage /out/; \
	"
