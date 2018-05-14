#[macro_use]
extern crate hippo_api;

#[macro_use]
extern crate rute;

use rute::rute::PushButton;
use rute::PluginUi;

use hippo_api::view::View;
use hippo_api::{Service, MessageApi, MessageDecode};

#[derive(Default)]
struct Player {
	message_api: MessageApi,
    ui: PluginUi,
    prev_button: PushButton,
    stop_button: PushButton,
    play_button: PushButton,
    next_button: PushButton,
}

impl View for Player {
    fn new(service: &Service) -> Player {
    	Player {
    		message_api: service.message_api(),
        	.. Player::default()
        }
    }

    fn setup_ui(&mut self, ui: PluginUi) {
        self.ui = ui;

        let buttons = ui.create_widget();

        self.prev_button = Self::create_button(&ui, "bin/player/buttons/hip_button_back.png");
        self.stop_button = Self::create_button(&ui, "bin/player/buttons/hip_button_stop.png");
        self.play_button = Self::create_button(&ui, "bin/player/buttons/hip_button_play.png");
        self.next_button = Self::create_button(&ui, "bin/player/buttons/hip_button_next.png");

        set_push_button_pressed_event!(self.prev_button, self, Player, Player::prev_song);
        set_push_button_pressed_event!(self.play_button, self, Player, Player::play_song);
        set_push_button_pressed_event!(self.stop_button, self, Player, Player::stop_song);
        set_push_button_pressed_event!(self.next_button, self, Player, Player::next_song);

        let vbox = ui.create_v_box_layout();
        let hbox = ui.create_h_box_layout();

        hbox.add_widget(&self.prev_button);
        hbox.add_widget(&self.stop_button);
        hbox.add_widget(&self.play_button);
        hbox.add_widget(&self.next_button);

        buttons.set_layout(&hbox);

        //vbox.add_widget(&self.player_display);
        vbox.add_widget(&buttons);

        ui.get_parent().set_layout(&vbox);
    }

    fn event(&mut self, event: &MessageDecode) {
    	println!("event: {}", event.get_method());
    }

    fn destroy(&mut self) {}
}

impl Player {
    fn create_button(ui: &PluginUi, icon_filename: &str) -> PushButton {
        let icon = ui.create_icon();
        let button = ui.create_push_button();

        icon.add_file(icon_filename);
        button.set_icon(&icon);

        button
    }

    fn prev_song(&mut self) {}

    fn play_song(&mut self) {}

    fn stop_song(&mut self) {}

    ///
    /// Sends a notification that next song should be started.
    /// Notice that this is not a request as a general "next playing song" will be sent out
    /// instead as this affects more plugins than just the sender
    /// 
    fn next_song(&mut self) {
    	self.message_api.next_song();
    }
}

#[no_mangle]
pub fn hippo_view_plugin() -> *const std::os::raw::c_void {
    define_view_plugin!(PLUGIN, b"Player\0", b"0.0.1\0", Player);
    // Would be nice to get rid of this
    let ret: *const std::os::raw::c_void = unsafe { std::mem::transmute(&PLUGIN) };
    ret
}
