struct Params {
    width: u32,
    height: u32,
    gas_count: u32,
    cell_count: u32,
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(0) @binding(1)
var<storage, read> permeability_mask: array<u32>;

@group(0) @binding(2)
var<storage, read> previous_values: array<u32>;

@group(0) @binding(3)
var<storage, read_write> next_values: array<u32>;

fn scalar_index(cell_index: u32, gas_index: u32) -> u32 {
    return cell_index * params.gas_count + gas_index;
}

fn pair_transfer(self_value: u32, neighbor_value: u32) -> u32 {
    if neighbor_value > self_value {
        return (neighbor_value - self_value) / 4u;
    }
    return (self_value - neighbor_value) / 4u;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let scalar_count = params.cell_count * params.gas_count;
    let scalar_id = global_id.x;
    if scalar_id >= scalar_count {
        return;
    }

    let gas_index = scalar_id % params.gas_count;
    let cell_index = scalar_id / params.gas_count;
    if permeability_mask[cell_index] == 0u {
        next_values[scalar_id] = 0u;
        return;
    }

    let x = cell_index % params.width;
    let y = cell_index / params.width;
    let self_value = previous_values[scalar_id];

    var gain: u32 = 0u;
    var loss: u32 = 0u;

    if x > 0u {
        let neighbor_cell = cell_index - 1u;
        if permeability_mask[neighbor_cell] != 0u {
            let neighbor_value = previous_values[scalar_index(neighbor_cell, gas_index)];
            let transfer = pair_transfer(self_value, neighbor_value);
            if neighbor_value > self_value {
                gain += transfer;
            } else if self_value > neighbor_value {
                loss += transfer;
            }
        }
    }

    if x + 1u < params.width {
        let neighbor_cell = cell_index + 1u;
        if permeability_mask[neighbor_cell] != 0u {
            let neighbor_value = previous_values[scalar_index(neighbor_cell, gas_index)];
            let transfer = pair_transfer(self_value, neighbor_value);
            if neighbor_value > self_value {
                gain += transfer;
            } else if self_value > neighbor_value {
                loss += transfer;
            }
        }
    }

    if y > 0u {
        let neighbor_cell = cell_index - params.width;
        if permeability_mask[neighbor_cell] != 0u {
            let neighbor_value = previous_values[scalar_index(neighbor_cell, gas_index)];
            let transfer = pair_transfer(self_value, neighbor_value);
            if neighbor_value > self_value {
                gain += transfer;
            } else if self_value > neighbor_value {
                loss += transfer;
            }
        }
    }

    if y + 1u < params.height {
        let neighbor_cell = cell_index + params.width;
        if permeability_mask[neighbor_cell] != 0u {
            let neighbor_value = previous_values[scalar_index(neighbor_cell, gas_index)];
            let transfer = pair_transfer(self_value, neighbor_value);
            if neighbor_value > self_value {
                gain += transfer;
            } else if self_value > neighbor_value {
                loss += transfer;
            }
        }
    }

    next_values[scalar_id] = (self_value + gain) - loss;
}
