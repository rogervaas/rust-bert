// Copyright 2020 The Google Research Authors.
// Copyright 2019-present, the HuggingFace Inc. team
// Copyright (c) 2018, NVIDIA CORPORATION.  All rights reserved.
// Copyright 2019 Guillaume Becquin
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


use rust_bert::resources::{Resource, download_resource, RemoteResource};
use rust_bert::electra::{ElectraConfig, ElectraForMaskedLM, ElectraModelResources, ElectraConfigResources, ElectraVocabResources};
use rust_bert::Config;
use rust_tokenizers::{BertTokenizer, Tokenizer, TruncationStrategy, Vocab};
use tch::{Tensor, Device, nn, no_grad};

fn main() -> failure::Fallible<()> {
    //    Resources paths
    let config_resource = Resource::Remote(RemoteResource::from_pretrained(ElectraConfigResources::BASE_GENERATOR));
    let vocab_resource = Resource::Remote(RemoteResource::from_pretrained(ElectraVocabResources::BASE_GENERATOR));
    let weights_resource = Resource::Remote(RemoteResource::from_pretrained(ElectraModelResources::BASE_GENERATOR));
    let config_path = download_resource(&config_resource)?;
    let vocab_path = download_resource(&vocab_resource)?;
    let weights_path = download_resource(&weights_resource)?;

//    Set-up masked LM model
    let device = Device::Cpu;
    let mut vs = nn::VarStore::new(device);
    let tokenizer: BertTokenizer = BertTokenizer::from_file(vocab_path.to_str().unwrap(), true);
    let config = ElectraConfig::from_file(config_path);
    let electra_model = ElectraForMaskedLM::new(&vs.root(), &config);
    vs.load(weights_path)?;

//    Define input
    let input = ["Looks like one [MASK] is missing", "It was a very nice and [MASK] day"];
    let tokenized_input = tokenizer.encode_list(input.to_vec(), 128, &TruncationStrategy::LongestFirst, 0);
    let max_len = tokenized_input.iter().map(|input| input.token_ids.len()).max().unwrap();
    let tokenized_input = tokenized_input.
        iter().
        map(|input| input.token_ids.clone()).
        map(|mut input| {
            input.extend(vec![0; max_len - input.len()]);
            input
        }).
        map(|input|
            Tensor::of_slice(&(input))).
        collect::<Vec<_>>();
    let input_tensor = Tensor::stack(tokenized_input.as_slice(), 0).to(device);

    //    Forward pass
    let (output, _, _) = no_grad(|| {
        electra_model
            .forward_t(Some(input_tensor),
                       None,
                       None,
                       None,
                       None,
                       false)
    });

//    Print masked tokens
    let index_1 = output.get(0).get(4).argmax(0, false);
    let index_2 = output.get(1).get(7).argmax(0, false);
    let word_1 = tokenizer.vocab().id_to_token(&index_1.int64_value(&[]));
    let word_2 = tokenizer.vocab().id_to_token(&index_2.int64_value(&[]));

    println!("{}", word_1); // Outputs "thing" : "Looks like one [thing] is missing"
    println!("{}", word_2);// Outputs "sunny" : "It was a very nice and [sunny] day"

    Ok(())
}
