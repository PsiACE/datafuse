use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use async_channel::{Sender, TrySendError};
use common_arrow::arrow::io::flight::serialize_batch;
use common_arrow::arrow_format::flight::data::FlightData;
use common_datablocks::DataBlock;
use common_exception::ErrorCode;
use crate::api::rpc::exchange::exchange_params::{SerializeParams, ShuffleExchangeParams};
use crate::api::rpc::flight_scatter::FlightScatter;
use crate::pipelines::new::processors::port::{InputPort, OutputPort};
use crate::pipelines::new::processors::Processor;
use crate::pipelines::new::processors::processor::{Event, ProcessorPtr};
use crate::sessions::QueryContext;
use common_exception::Result;
use crate::api::rpc::exchange::exchange_channel::FragmentSender;
use crate::api::rpc::packet::DataPacket;

struct OutputData {
    pub data_block: Option<DataBlock>,
    pub serialized_blocks: Vec<Option<DataPacket>>,
}

pub struct ExchangePublisherSink<const HAS_OUTPUT: bool> {
    ctx: Arc<QueryContext>,
    fragment_id: usize,

    serialize_params: SerializeParams,
    shuffle_exchange_params: ShuffleExchangeParams,

    initialize: bool,
    input: Arc<InputPort>,
    output: Arc<OutputPort>,
    input_data: Option<DataBlock>,
    output_data: Option<OutputData>,
    peer_endpoint_publisher: Vec<FragmentSender>,
}

impl<const HAS_OUTPUT: bool> ExchangePublisherSink<HAS_OUTPUT> {
    pub fn try_create(
        ctx: Arc<QueryContext>,
        fragment_id: usize,
        input: Arc<InputPort>,
        output: Arc<OutputPort>,
        shuffle_exchange_params: ShuffleExchangeParams,
    ) -> Result<ProcessorPtr> {
        let serialize_params = shuffle_exchange_params.create_serialize_params()?;
        Ok(ProcessorPtr::create(Box::new(ExchangePublisherSink::<HAS_OUTPUT> {
            ctx,
            input,
            output,
            fragment_id,
            serialize_params,
            shuffle_exchange_params,
            initialize: false,
            input_data: None,
            output_data: None,
            peer_endpoint_publisher: vec![],
        })))
    }

    fn get_peer_endpoint_publisher(&self) -> Result<Vec<FragmentSender>> {
        let destination_ids = &self.shuffle_exchange_params.destination_ids;
        let mut res = Vec::with_capacity(destination_ids.len());
        let exchange_manager = self.ctx.get_exchange_manager();

        for destination_id in destination_ids {
            if destination_id != &self.shuffle_exchange_params.executor_id {
                let id = self.fragment_id;
                let query_id = &self.shuffle_exchange_params.query_id;
                res.push(exchange_manager.get_fragment_sink(query_id, id, destination_id)?);
            } else if !HAS_OUTPUT {
                return Err(ErrorCode::LogicalError(
                    "Has local output, but not found output port. It's a bug."
                ));
            }
        }

        Ok(res)
    }
}

#[async_trait::async_trait]
impl<const HAS_OUTPUT: bool> Processor for ExchangePublisherSink<HAS_OUTPUT> {
    fn name(&self) -> &'static str {
        "ExchangePublisher"
    }

    fn event(&mut self) -> Result<Event> {
        if !self.initialize {
            self.initialize = true;
            self.peer_endpoint_publisher = self.get_peer_endpoint_publisher()?;
        }

        if HAS_OUTPUT {
            if self.output.is_finished() {
                self.input.finish();
                self.peer_endpoint_publisher.clear();
                return Ok(Event::Finished);
            }

            // This may cause other cluster nodes to idle.
            if !self.output.can_push() {
                self.input.set_not_need_data();
                return Ok(Event::NeedConsume);
            }
        }


        if let Some(mut output_data) = self.output_data.take() {
            let mut pushed_data = false;
            if HAS_OUTPUT {
                // If has local data block, the push block to the output port
                if let Some(data_block) = output_data.data_block.take() {
                    pushed_data = true;
                    self.output.push_data(Ok(data_block));
                }
            }

            // When the sender is fast enough, we can try to send. If all of them are sent successfully, it will reduce the scheduling of the processor once.
            let mut need_async_send = false;
            for (index, publisher) in self.peer_endpoint_publisher.iter().enumerate() {
                if output_data.serialized_blocks[index].is_some() {
                    let data = output_data.serialized_blocks[index].take().unwrap();
                    match publisher.try_send(data) {
                        Ok(_) => { /* do nothing*/ }
                        Err(TrySendError::Closed(_)) => {
                            if HAS_OUTPUT {
                                self.output.finish();
                            }

                            self.peer_endpoint_publisher.clear();
                            return Ok(Event::Finished);
                        }
                        Err(TrySendError::Full(value)) => {
                            need_async_send = true;
                            output_data.serialized_blocks[index] = Some(value);
                        }
                    }
                }
            }

            if need_async_send {
                self.output_data = Some(output_data);
                return Ok(Event::Async);
            }

            if HAS_OUTPUT && pushed_data {
                return Ok(Event::NeedConsume);
            }
        }

        if self.input_data.is_some() {
            return Ok(Event::Sync);
        }

        if self.input.is_finished() {
            if HAS_OUTPUT {
                self.output.finish();
            }

            self.peer_endpoint_publisher.clear();
            return Ok(Event::Finished);
        }

        if self.input.has_data() {
            self.input_data = Some(self.input.pull_data().unwrap()?);
            return Ok(Event::Sync);
        }

        self.input.set_need_data();
        Ok(Event::NeedData)
    }

    fn process(&mut self) -> Result<()> {
        if let Some(data_block) = self.input_data.take() {
            let scatter = &self.shuffle_exchange_params.shuffle_scatter;

            let scatted_blocks = scatter.execute(&data_block, 0)?;
            let mut output_data = OutputData { data_block: None, serialized_blocks: vec![] };

            for (index, data_block) in scatted_blocks.into_iter().enumerate() {
                if data_block.is_empty() {
                    output_data.serialized_blocks.push(None);
                    continue;
                }

                if HAS_OUTPUT && index == self.serialize_params.local_executor_pos {
                    output_data.data_block = Some(data_block);
                    output_data.serialized_blocks.push(None);
                } else {
                    let chunks = data_block.try_into()?;
                    let options = &self.serialize_params.options;
                    let ipc_fields = &self.serialize_params.ipc_fields;
                    let (dicts, values) = serialize_batch(&chunks, ipc_fields, options);

                    if !dicts.is_empty() {
                        return Err(ErrorCode::UnImplement("DatabendQuery does not implement dicts."));
                    }

                    output_data.serialized_blocks.push(Some(DataPacket::Data(self.fragment_id, values)));
                }
            }

            self.output_data = Some(output_data);
        }

        Ok(())
    }

    async fn async_process(&mut self) -> Result<()> {
        if let Some(mut output_data) = self.output_data.take() {
            for (index, publisher) in self.peer_endpoint_publisher.iter().enumerate() {
                if let Some(flight_data) = output_data.serialized_blocks[index].take() {
                    if let Err(_) = publisher.send(flight_data).await {
                        return Err(ErrorCode::TokioError(
                            "Cannot send flight data to endpoint, because sender is closed."
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
