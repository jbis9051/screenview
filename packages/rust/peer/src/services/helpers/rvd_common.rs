macro_rules! clipboard_request_impl {
    ($self:ident, $msg: ident, $send:ident, $error_type:ty) => {{
        if !$self.permissions().clipboard_readable {
            return Err(<$error_type>::PermissionsError("read clipboard".to_owned()));
        }
        let clip_type = get_native_clipboard(&$msg.info.clipboard_type);
        let content = $self.native.clipboard_content(&clip_type).ok();
        $send
            .send(ScreenViewMessage::RvdMessage(
                RvdMessage::ClipboardNotification(ClipboardNotification {
                    info: $msg.info.clone(),
                    content: if $msg.info.content_request {
                        content
                    } else {
                        None
                    },
                }),
            ))
            .map_err(<$error_type>::SendError)?;
        Ok(())
    }};
}

macro_rules! clipboard_notificaiton_impl {
    ($self:ident, $msg: ident, $error_type:ty) => {{
        if !$self.permissions().clipboard_writable {
            return Err(<$error_type>::PermissionsError(
                "write clipboard".to_owned(),
            ));
        }
        let clip_type = get_native_clipboard(&$msg.info.clipboard_type);
        if !$msg.info.content_request {
            // TODO handle
            return Ok(());
        }
        $self
            .native
            .set_clipboard_content(&clip_type, &$msg.content.unwrap())
            .map_err(<$error_type>::NativeError)?;
        Ok(())
    }};
}

pub(crate) use clipboard_notificaiton_impl;
pub(crate) use clipboard_request_impl;
