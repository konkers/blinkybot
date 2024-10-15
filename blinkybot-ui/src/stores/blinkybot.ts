import { ref, computed } from 'vue'
import { defineStore } from 'pinia'

export const useBlinkyBotStore = defineStore('blinkybot', {
	state: (): BlinkyBot => {
		return { device: null as USBDevice | null }
	},
	getters: {
		isConnected(): boolean {
			return this.device != null;
		}
	},
	actions: {
		async connect() {
			if (this.device !== null) {
				return;
			}

			const device = await navigator.usb.requestDevice({ filters: [{ vendorId: 0xf569 }] })
			await device.open()
			await device.claimInterface(1)
			this.device = device;
		},

		async disconnect() {
			if (this.device === null) {
				return;
			}

			await this.device.close()
			this.device = null;
		},

		async testTransaction() {
			if (this.device === null) {
				return;
			}
			this.device.transferIn(1, 64).then((data) => console.log(data))
			await this.device.transferOut(1, new Uint8Array([1, 2, 3]))
		}
	},
})

interface BlinkyBot {
	device: USBDevice | null;
}
