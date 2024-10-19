use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_rp::flash::{Async, Flash};
use embassy_rp::peripherals::{FLASH, USB};
use embassy_rp::usb::{Driver as UsbDriver, Endpoint, Out};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::{DynReceiver, DynSender, Watch};
use embassy_usb::class::web_usb::{Config as WebUsbConfig, State, Url, WebUsb};
use embassy_usb::driver::Driver;
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Config, UsbDevice};

use postcard_rpc::{
    define_dispatch,
    target_server::{buffers::AllBuffers, rpc_dispatch, SpawnContext},
    WireHeader,
};

use blinkybot_rpc::{
    Expression, ExpressionIndex, GetAdcEndpoint, GetBrightnessEndpoint, GetExpressionEndpoint,
    PingEndpoint, SetBrightnessEndpoint, SetExpression, SetExpressionEndpoint,
};
use static_cell::{ConstStaticCell, StaticCell};

use crate::config_store::FlashConfigStore;

pub struct Comms {
    pub default_expression: Watch<ThreadModeRawMutex, Expression, 1>,
    pub blink_expression: Watch<ThreadModeRawMutex, Expression, 1>,
    pub friend_expression: Watch<ThreadModeRawMutex, Expression, 1>,
    pub friend_blink_expression: Watch<ThreadModeRawMutex, Expression, 1>,
    pub adc_val: Watch<ThreadModeRawMutex, u16, 2>,
    pub brightness_val: Watch<ThreadModeRawMutex, u8, 1>,
}

impl Comms {
    pub fn new() -> Self {
        Self {
            default_expression: Watch::new(),
            blink_expression: Watch::new(),
            friend_expression: Watch::new(),
            friend_blink_expression: Watch::new(),
            adc_val: Watch::new(),
            brightness_val: Watch::new(),
        }
    }
}

pub struct Context {
    default_expression_sender: DynSender<'static, Expression>,
    blink_expression_sender: DynSender<'static, Expression>,
    friend_expression_sender: DynSender<'static, Expression>,
    friend_blink_expression_sender: DynSender<'static, Expression>,
    adc_val_receiver: DynReceiver<'static, u16>,
    brightness_val_sender: DynSender<'static, u8>,
    config_store: FlashConfigStore<Flash<'static, FLASH, Async, { crate::FLASH_SIZE }>>,
}

pub struct SpawnCtx {}

impl SpawnContext for Context {
    type SpawnCtxt = SpawnCtx;
    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        SpawnCtx {}
    }
}
define_dispatch! {
    dispatcher: Dispatcher<
        Mutex = ThreadModeRawMutex,
        Driver = UsbDriver<'static, USB>,
        Context = Context,
    >;
    PingEndpoint => blocking ping_handler,
    SetExpressionEndpoint => async set_expression_handler,
    GetExpressionEndpoint => async get_expression_handler,
    GetAdcEndpoint => async get_adc_handler,
    GetBrightnessEndpoint => async get_brightness_handler,
    SetBrightnessEndpoint => async set_brightness_handler,
}

static ALL_BUFFERS: ConstStaticCell<AllBuffers<256, 256, 256>> =
    ConstStaticCell::new(AllBuffers::new());

// This is a randomly generated GUID to allow clients on Windows to find our device
const DEVICE_INTERFACE_GUIDS: &[&str] = &["{753ff41c-07a6-48f2-8655-abcdbe4a4cab}"];

pub async fn setup(
    spawner: Spawner,
    driver: UsbDriver<'static, USB>,
    config_store: FlashConfigStore<Flash<'static, FLASH, Async, { crate::FLASH_SIZE }>>,
) -> &'static Comms {
    // Create embassy-usb Config
    let mut config = Config::new(0xf569, 0x0001);
    config.manufacturer = Some("Konkers");
    config.product = Some("BlinkyBot");
    config.serial_number = Some("12345678");
    config.max_power = 500;
    config.max_packet_size_0 = 64;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xff;
    config.device_sub_class = 0x00;
    config.device_protocol = 0x00;

    static WEB_USB_CONFIG: StaticCell<WebUsbConfig> = StaticCell::new();
    let webusb_config: &mut WebUsbConfig = WEB_USB_CONFIG.init(WebUsbConfig {
        max_packet_size: 64,
        vendor_code: 1,
        // If defined, shows a landing page which the device manufacturer would like the user to visit in order to control their device. Suggest the user to navigate to this URL when the device is connected.
        landing_url: Some(Url::new("http://localhost:8080")),
    });

    static COMMS: StaticCell<Comms> = StaticCell::new();
    let comms: &Comms = COMMS.init(Comms::new());

    static STATE: StaticCell<State> = StaticCell::new();
    let state: &mut State = STATE.init(State::new());

    let buffers = ALL_BUFFERS.take();
    let mut builder = Builder::new(
        driver,
        config,
        &mut buffers.usb_device.config_descriptor,
        &mut buffers.usb_device.bos_descriptor,
        &mut buffers.usb_device.msos_descriptor,
        &mut buffers.usb_device.control_buf,
    );

    // Add the Microsoft OS Descriptor (MSOS/MOD) descriptor.
    // We tell Windows that this entire device is compatible with the "WINUSB" feature,
    // which causes it to use the built-in WinUSB driver automatically, which in turn
    // can be used by libusb/rusb software without needing a custom driver or INF file.
    // In principle you might want to call msos_feature() just on a specific function,
    // if your device also has other functions that still use standard class drivers.
    builder.msos_descriptor(windows_version::WIN8_1, 0);
    builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(DEVICE_INTERFACE_GUIDS),
    ));

    // Create classes on the builder (WebUSB just needs some setup, but doesn't return anything)
    WebUsb::configure(&mut builder, state, webusb_config);
    // Create some USB bulk endpoints for testing.
    let endpoints = WebEndpoints::new(&mut builder, webusb_config);

    // Build the builder.
    let usb = builder.build();

    let mut context = Context {
        default_expression_sender: comms.default_expression.dyn_sender(),
        blink_expression_sender: comms.blink_expression.dyn_sender(),
        friend_expression_sender: comms.friend_expression.dyn_sender(),
        friend_blink_expression_sender: comms.friend_blink_expression.dyn_sender(),
        adc_val_receiver: comms.adc_val.dyn_receiver().unwrap(),
        brightness_val_sender: comms.brightness_val.dyn_sender(),
        config_store,
    };
    context.default_expression_sender.send(
        context
            .config_store
            .get_expression(ExpressionIndex::Default)
            .await,
    );
    context.blink_expression_sender.send(
        context
            .config_store
            .get_expression(ExpressionIndex::Blink)
            .await,
    );
    context.friend_expression_sender.send(
        context
            .config_store
            .get_expression(ExpressionIndex::Friend)
            .await,
    );
    context.friend_blink_expression_sender.send(
        context
            .config_store
            .get_expression(ExpressionIndex::FriendBlink)
            .await,
    );
    context
        .brightness_val_sender
        .send(context.config_store.get_brightness().await);
    let dispatch = Dispatcher::new(&mut buffers.tx_buf, endpoints.write_ep, context);

    spawner.must_spawn(dispatch_task(
        endpoints.read_ep,
        dispatch,
        &mut buffers.rx_buf,
    ));
    spawner.must_spawn(usb_task(usb));

    comms
}

struct WebEndpoints<'d, D: Driver<'d>> {
    write_ep: D::EndpointIn,
    read_ep: D::EndpointOut,
}

impl<'d, D: Driver<'d>> WebEndpoints<'d, D> {
    fn new(builder: &mut Builder<'d, D>, config: &'d WebUsbConfig<'d>) -> Self {
        let mut func = builder.function(0xff, 0x00, 0x00);
        let mut iface = func.interface();
        let mut alt = iface.alt_setting(0xff, 0x00, 0x00, None);

        let write_ep = alt.endpoint_bulk_in(config.max_packet_size);
        let read_ep = alt.endpoint_bulk_out(config.max_packet_size);

        WebEndpoints { write_ep, read_ep }
    }
}

/// This actually runs the dispatcher
#[embassy_executor::task]
async fn dispatch_task(
    ep_out: Endpoint<'static, USB, Out>,
    dispatch: Dispatcher,
    rx_buf: &'static mut [u8],
) {
    rpc_dispatch(ep_out, dispatch, rx_buf).await;
}

/// This handles the low level USB management
#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, UsbDriver<'static, USB>>) {
    usb.run().await;
}

fn ping_handler(_context: &mut Context, header: WireHeader, rqst: u32) -> u32 {
    info!("ping: seq - {=u32}", header.seq_no);
    rqst
}

async fn set_expression_handler(context: &mut Context, header: WireHeader, request: SetExpression) {
    info!("set expression: seq - {=u32} {}", header.seq_no, request);
    if let Err(e) = context
        .config_store
        .set_expression(request.index.clone(), request.expression.clone())
        .await
    {
        error!(
            "Failed to save espression {} to flash: {}",
            request.index, e
        );
    }
    match &request.index {
        blinkybot_rpc::ExpressionIndex::Default => {
            context.default_expression_sender.send(request.expression)
        }
        blinkybot_rpc::ExpressionIndex::Blink => {
            context.blink_expression_sender.send(request.expression)
        }
        blinkybot_rpc::ExpressionIndex::Friend => {
            context.friend_expression_sender.send(request.expression)
        }
        blinkybot_rpc::ExpressionIndex::FriendBlink => context
            .friend_blink_expression_sender
            .send(request.expression),
    }
}

async fn get_expression_handler(
    context: &mut Context,
    header: WireHeader,
    request: ExpressionIndex,
) -> Expression {
    info!("get expression: seq - {=u32} {}", header.seq_no, request);
    context.config_store.get_expression(request).await
}

async fn get_adc_handler(context: &mut Context, header: WireHeader, _request: ()) -> u16 {
    info!("get adc: seq - {=u32}", header.seq_no);

    context.adc_val_receiver.get().await
}

async fn get_brightness_handler(context: &mut Context, header: WireHeader, _request: ()) -> u8 {
    let val = context.config_store.get_brightness().await;
    info!("get brightness: seq - {=u32} {}", header.seq_no, val);
    val
}

async fn set_brightness_handler(context: &mut Context, header: WireHeader, request: u8) {
    info!("set brightness: seq - {=u32} {}", header.seq_no, request);

    if let Err(e) = context.config_store.set_brightness(request).await {
        error!("Failed to save brightness to flash: {}", e);
    }
    context.brightness_val_sender.send(request);
}
