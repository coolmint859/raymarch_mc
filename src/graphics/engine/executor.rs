use crate::graphics::{GpuCommand, GpuContext};

/// Represents the execution phase of gpu operations.
pub trait SequentialExecutor {
    /// Add a command to the executor
    fn add_command(&mut self, cmd: impl GpuCommand + 'static);

    /// Record/submit all known commands and execute them on the gpu
    fn submit<'a>(&mut self, context: &'a GpuContext);
}

/// Records and executes multiple command buffers sequentially.
pub struct MultiBufferExecutor {
    commands: Vec<Box<dyn GpuCommand>>,
    recordings: Vec<wgpu::CommandBuffer>,
}

impl MultiBufferExecutor {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            recordings: Vec::new()
        }
    }

    /// Record all known commands into a command buffer. This resets the command queue.
    pub fn record<'a>(&mut self, context: &'a GpuContext) {
        let mut encoder = context.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let commands = std::mem::take(&mut self.commands);
        for mut cmd in commands {
            cmd.record(&mut encoder, context);
        }

        self.recordings.push(encoder.finish());
    }

    /// Record and Execute commands on the gpu
    pub fn record_and_submit<'a>(&mut self, context: &'a GpuContext) {
        self.record(context);
        self.submit(context);
    }
}

impl SequentialExecutor for MultiBufferExecutor {
    fn add_command(&mut self, cmd: impl GpuCommand + 'static) {
        self.commands.push(Box::new(cmd));
    }

    fn submit<'a>(&mut self, context: &'a GpuContext) {
        context.gpu.queue.submit(std::mem::take(&mut self.recordings));
    }
}
