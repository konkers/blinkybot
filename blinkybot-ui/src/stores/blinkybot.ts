import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import init, { greet, BlinkyBotClient, ExpressionIndex, Expression } from 'blinkybot-ui-wasm';

export { Expression, ExpressionIndex } from 'blinkybot-ui-wasm';

export const useBlinkyBotStore = defineStore('blinkybot', {
	state: (): BlinkyBot => {
		return { wasmInitialized: false, client: null }
	},
	getters: {
		isConnected(): boolean {
			return this.client != null;
		}
	},
	actions: {
		async connect() {
			if (!this.wasmInitialized) {
				await init();
				this.wasmInitialized = true;
			}

			if (this.client !== null) {
				return;
			}

			const client = await new BlinkyBotClient();
			this.client = client;
		},

		async disconnect() {
			if (this.client === null) {
				return;
			}

			this.client.close();
			await this.client.wait_closed();
			this.client = null;
		},

		async ping(id: number): Promise<number> {
			console.log(this.client);
			if (this.client === null) {
				return 0;
			}
			return await this.client.ping(1);
		},

		async set_expression(index: ExpressionIndex, expression: Expression) {
			if (this.client === null) {
				return;
			}
			await this.client.set_expression(index, expression);
		}
	},
})

interface BlinkyBot {
	client: BlinkyBotClient | null;
	wasmInitialized: boolean;
}
