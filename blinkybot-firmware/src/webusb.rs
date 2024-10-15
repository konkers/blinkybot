use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, Endpoint, Out};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_usb::class::web_usb::{Config as WebUsbConfig, State, Url, WebUsb};
use embassy_usb::driver::Driver;
use embassy_usb::msos::{self, windows_version};
use embassy_usb::{Builder, Config, UsbDevice};

use postcard_rpc::{
    define_dispatch,
    target_server::{buffers::AllBuffers, rpc_dispatch, SpawnContext},
    WireHeader,
};

use blinkybot_rpc::PingEndpoint;
use static_cell::{ConstStaticCell, StaticCell};

pub struct Context {}

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
}

static ALL_BUFFERS: ConstStaticCell<AllBuffers<256, 256, 256>> =
    ConstStaticCell::new(AllBuffers::new());

// This is a randomly generated GUID to allow clients on Windows to find our device
const DEVICE_INTERFACE_GUIDS: &[&str] = &["{753ff41c-07a6-48f2-8655-abcdbe4a4cab}"];

pub fn setup(spawner: Spawner, driver: UsbDriver<'static, USB>) {
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

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    // let mut config_descriptor = [0; 256];
    // let mut bos_descriptor = [0; 256];
    // let mut control_buf = [0; 64];
    // let mut msos_descriptor = [0; 256];

    static WEB_USB_CONFIG: StaticCell<WebUsbConfig> = StaticCell::new();
    let webusb_config: &mut WebUsbConfig = WEB_USB_CONFIG.init(WebUsbConfig {
        max_packet_size: 64,
        vendor_code: 1,
        // If defined, shows a landing page which the device manufacturer would like the user to visit in order to control their device. Suggest the user to navigate to this URL when the device is connected.
        landing_url: Some(Url::new("http://localhost:8080")),
    });

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

    let dispatch = Dispatcher::new(&mut buffers.tx_buf, endpoints.write_ep, Context {});

    spawner.must_spawn(dispatch_task(
        endpoints.read_ep,
        dispatch,
        &mut buffers.rx_buf,
    ));
    spawner.must_spawn(usb_task(usb));
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
